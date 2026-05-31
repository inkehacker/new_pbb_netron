use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Instant;
use std::fs::File;

use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rand::Rng;

use crate::namecalc::{N, load_team, median3, fill_full_namebase, calc_skills, loading_name, compute_props, SkillSlot};
use crate::model::{MODEL, MODELQD, hanxu_poly};

const MAX_QUEUE_LEN: usize = 4;
const NUMBER_PER_TASK: u64 = 1_000_000;

#[derive(Clone)]
pub struct TaskData {
    pub task_id: usize,
    pub l: u64,
    pub r: u64,
    pub task_type: u8,
    pub prefix_id: usize,
    pub suffix_id: usize,
}

struct TaskQueueInner {
    data: [Option<TaskData>; MAX_QUEUE_LEN],
    head: usize,
    tail: usize,
    closed: bool,
}

pub struct MinerConfig {
    pub team_name: Vec<i8>,
    pub team_name_display: String,
    pub prefixes: Vec<Vec<i8>>,
    pub prefix_lens: Vec<usize>,
    pub suffixes: Vec<Vec<i8>>,
    pub suffix_lens: Vec<usize>,
    pub charset: Vec<i8>,
    pub charset_size: usize,
    pub single_char_len: usize,
    pub variable_len: usize,
    pub task_type: u8,
    pub sequential_ranges: Vec<(u64, u64)>,
    pub random_total: u64,
    pub xp_min: i32,
    pub xd_min: i32,
    pub collect_mode: u8,
    pub collect_8v_min: i32,
    pub collect_7v_min: i32,
    pub collect_hl_min: i32,
    pub collect_hp8v_min: i32,
    pub output_xp: bool,
    pub output_utf: bool,
    pub num_threads: usize,
    pub output_file: String,
}

pub fn run_miner(config: &MinerConfig) {
    std::fs::create_dir_all("./out").ok();

    let num_threads = config.num_threads;
    let all_totnum: u128 = if config.task_type == 1 {
        config.sequential_ranges.iter().map(|(l, r)| (r - l) as u128).sum::<u128>()
    } else {
        config.random_total as u128
    };

    let queue = Arc::new((Mutex::new(TaskQueueInner {
        data: std::array::from_fn(|_| None),
        head: 0,
        tail: 0,
        closed: false,
    }), Condvar::new(), Condvar::new()));

    let shared = Arc::new(Mutex::new(SharedState {
        task_finished_number: 0,
        tot_cnt: 0,
        max_sum: 0i32,
        max_xp: 0i32,
        max_xd: 0i32,
        all_totnum,
        start_time: Instant::now(),
    }));

    let output_file = Arc::new(Mutex::new(
        BufWriter::new(File::create(&config.output_file).expect("Cannot create output file"))
    ));

    let config = Arc::new(config.clone_for_threads());

    let mut threads = Vec::new();

    // Producer thread
    let q_prod = Arc::clone(&queue);
    let cfg_prod = Arc::clone(&config);
    let prod = thread::spawn(move || {
        producer(&q_prod, &cfg_prod);
    });
    threads.push(prod);

    // Consumer threads
    for thread_idx in 0..num_threads {
        let q_cons = Arc::clone(&queue);
        let s = Arc::clone(&shared);
        let cfg = Arc::clone(&config);
        let out = Arc::clone(&output_file);

        let cons = thread::spawn(move || {
            consumer(&q_cons, &s, &cfg, thread_idx, &out);
        });
        threads.push(cons);
    }

    for t in threads {
        let _ = t.join();
    }
    eprintln!("Done.");
}

#[derive(Clone)]
struct ThreadConfig {
    pub team_name: Vec<i8>,
    pub prefixes: Vec<Vec<i8>>,
    pub prefix_lens: Vec<usize>,
    pub suffixes: Vec<Vec<i8>>,
    pub suffix_lens: Vec<usize>,
    pub charset: Vec<i8>,
    pub charset_size: usize,
    pub single_char_len: usize,
    pub variable_len: usize,
    pub task_type: u8,
    pub sequential_ranges: Vec<(u64, u64)>,
    pub random_total: u64,
    pub xp_min: i32,
    pub xd_min: i32,
    pub collect_mode: u8,
    pub collect_8v_min: i32,
    pub collect_7v_min: i32,
    pub collect_hl_min: i32,
    pub collect_hp8v_min: i32,
    pub output_xp: bool,
    pub output_utf: bool,
}

impl MinerConfig {
    fn clone_for_threads(&self) -> ThreadConfig {
        ThreadConfig {
            team_name: self.team_name.clone(),
            prefixes: self.prefixes.clone(),
            prefix_lens: self.prefix_lens.clone(),
            suffixes: self.suffixes.clone(),
            suffix_lens: self.suffix_lens.clone(),
            charset: self.charset.clone(),
            charset_size: self.charset_size,
            single_char_len: self.single_char_len,
            variable_len: self.variable_len,
            task_type: self.task_type,
            sequential_ranges: self.sequential_ranges.clone(),
            random_total: self.random_total,
            xp_min: self.xp_min,
            xd_min: self.xd_min,
            collect_mode: self.collect_mode,
            collect_8v_min: self.collect_8v_min,
            collect_7v_min: self.collect_7v_min,
            collect_hl_min: self.collect_hl_min,
            collect_hp8v_min: self.collect_hp8v_min,
            output_xp: self.output_xp,
            output_utf: self.output_utf,
        }
    }
}

struct SharedState {
    task_finished_number: u64,
    tot_cnt: u64,
    max_sum: i32,
    max_xp: i32,
    max_xd: i32,
    all_totnum: u128,
    start_time: Instant,
}

fn producer(queue: &Arc<(Mutex<TaskQueueInner>, Condvar, Condvar)>, config: &ThreadConfig) {
    let (lock, add_cond, get_cond) = &**queue;
    let mut task_cnt: usize = 0;

    {
        let mut q = lock.lock().unwrap();
        if config.task_type == 1 {
            for (j, &(l, r)) in config.sequential_ranges.iter().enumerate() {
                for i in (l..r).step_by(NUMBER_PER_TASK as usize) {
                    let end = (i + NUMBER_PER_TASK).min(r);
                    let data = TaskData {
                        task_id: task_cnt,
                        l: i, r: end,
                        task_type: 1,
                        prefix_id: j,
                        suffix_id: j % config.suffixes.len(),
                    };
                    while !add_to_queue(&mut q, data.clone()) {
                        if q.closed { break; }
                        q = add_cond.wait(q).unwrap();
                    }
                    get_cond.notify_one();
                    task_cnt += 1;
                }
            }
        } else if config.task_type == 4 {
            let mut rng = ChaCha20Rng::from_entropy();
            for i in (0..config.random_total).step_by(NUMBER_PER_TASK as usize) {
                let end = (i + NUMBER_PER_TASK).min(config.random_total);
                let prefix_id = rng.gen_range(0..config.prefixes.len());
                let data = TaskData {
                    task_id: task_cnt,
                    l: i, r: end,
                    task_type: 2,
                    prefix_id,
                    suffix_id: prefix_id,
                };
                while !add_to_queue(&mut q, data.clone()) {
                    if q.closed { break; }
                    q = add_cond.wait(q).unwrap();
                }
                get_cond.notify_one();
                task_cnt += 1;
            }
        } else {
            let mut rng = ChaCha20Rng::from_entropy();
            for i in (0..config.random_total).step_by(NUMBER_PER_TASK as usize) {
                let end = (i + NUMBER_PER_TASK).min(config.random_total);
                let prefix_id = rng.gen_range(0..config.prefixes.len());
                let suffix_id = rng.gen_range(0..config.suffixes.len());
                let data = TaskData {
                    task_id: task_cnt,
                    l: i, r: end,
                    task_type: config.task_type,
                    prefix_id,
                    suffix_id,
                };
                while !add_to_queue(&mut q, data.clone()) {
                    if q.closed { break; }
                    q = add_cond.wait(q).unwrap();
                }
                get_cond.notify_one();
                task_cnt += 1;
            }
        }
        q.closed = true;
    }
    get_cond.notify_all();
}

fn add_to_queue(q: &mut TaskQueueInner, data: TaskData) -> bool {
    if q.closed { return false; }
    if q.tail.wrapping_sub(q.head) >= MAX_QUEUE_LEN { return false; }
    let idx = q.tail % MAX_QUEUE_LEN;
    q.tail += 1;
    q.data[idx] = Some(data);
    true
}

fn get_task(queue: &Arc<(Mutex<TaskQueueInner>, Condvar, Condvar)>) -> Option<TaskData> {
    let (lock, add_cond, get_cond) = &**queue;
    let mut q = lock.lock().unwrap();
    loop {
        if q.head < q.tail {
            let idx = q.head % MAX_QUEUE_LEN;
            q.head += 1;
            let task = q.data[idx].take();
            add_cond.notify_one();
            return task;
        }
        if q.closed { return None; }
        q = get_cond.wait(q).unwrap();
    }
}

fn consumer(
    queue: &Arc<(Mutex<TaskQueueInner>, Condvar, Condvar)>,
    shared: &Arc<Mutex<SharedState>>,
    config: &ThreadConfig,
    _thread_idx: usize,
    output_file: &Arc<Mutex<BufWriter<File>>>,
) {
    let scl = config.single_char_len;
    let charset_size = config.charset_size;
    let variable_len = config.variable_len;
    let prefix_count = config.prefixes.len();
    let suffix_count = config.suffixes.len();

    let model = MODEL;
    let model_qd = MODELQD;
    let xd_min = config.xd_min as f64;

    let mut val_base = [0u8; N];
    load_team(&mut val_base, &config.team_name);

    let mut rng = ChaCha20Rng::from_entropy();
    let mut local_tot: u64 = 0;
    let mut local_max_sum: i32 = 0;
    let mut local_max_xp: i32 = 0;
    let mut local_max_xd: i32 = 0;

    loop {
        let task = match get_task(queue) {
            Some(t) => t,
            None => break,
        };

        let prefix = &config.prefixes[task.prefix_id];
        let prefix_len = config.prefix_lens[task.prefix_id];
        let suffix = &config.suffixes[task.suffix_id];
        let suffix_len = config.suffix_lens[task.suffix_id];

        let mut tmp = vec![0i8; 1024];
        for i in 0..prefix_len { tmp[i] = prefix[i]; }
        let full_var_start = prefix_len;
        for i in 0..suffix_len {
            tmp[full_var_start + variable_len * scl + i] = suffix[i];
        }

        let mut prelen = prefix_len;
        let mut varlen = variable_len;
        let mut random_range_max: u64 = 1;

        if task.task_type == 2 || task.task_type == 3 {
            let mut x: u64 = 1;
            let mut __varlen = 0;
            while x < NUMBER_PER_TASK {
                __varlen += 1;
                x = x.saturating_mul(charset_size as u64);
            }
            varlen = __varlen;
            random_range_max = x;
            prelen = prefix_len + (variable_len - varlen) * scl;

            // Fill the extended prefix with random charset chars
            for pos in (prefix_len..prelen).step_by(scl) {
                let mut lastrnd: u64 = 0;
                if lastrnd <= charset_size as u64 { lastrnd = rng.gen(); }
                let cur = (lastrnd % charset_size as u64) as usize;
                for ii in 0..scl {
                    tmp[pos + ii] = config.charset[cur * scl + ii];
                }
                lastrnd /= charset_size as u64;
            }
        }

        let (l_start, r_end): (u64, u64);
        if task.task_type == 2 {
            l_start = rng.gen_range(0..random_range_max);
            r_end = l_start + NUMBER_PER_TASK;
        } else {
            l_start = task.l;
            r_end = task.r;
        }

        let name_len = prelen + varlen * scl;
        let total_len_for_shuffle = name_len; // This is what C++ uses as NAMELEN

        process_names(
            &mut rng,
            &mut tmp, prelen, varlen, scl,
            charset_size, &config.charset,
            l_start, r_end, task.task_type,
            name_len, total_len_for_shuffle,
            &val_base,
            &model, &model_qd, config, xd_min,
            output_file,
            &mut local_tot, &mut local_max_sum, &mut local_max_xp, &mut local_max_xd,
        );

        {
            let mut s = shared.lock().unwrap();
            s.task_finished_number += 1;
            s.tot_cnt += local_tot;
            if local_max_sum > s.max_sum { s.max_sum = local_max_sum; }
            if local_max_xp > s.max_xp { s.max_xp = local_max_xp; }
            if local_max_xd > s.max_xd { s.max_xd = local_max_xd; }

            if s.task_finished_number % 100 == 0 {
                let elapsed = s.start_time.elapsed().as_secs_f64();
                let speed = (s.task_finished_number * NUMBER_PER_TASK) as f64 / elapsed / 1e12 * 86400.0;
                let remaining = s.all_totnum.saturating_sub((s.task_finished_number * (NUMBER_PER_TASK as u64)) as u128);
                let time_left_secs = if speed > 0.0 {
                    (remaining as f64 / (speed * 1e12 / 86400.0)) as u64
                } else { 0 };
                eprintln!("task{} finished, count:{:.6}T", task.task_id,
                    s.task_finished_number as f64 * NUMBER_PER_TASK as f64 / 1e12);
                eprintln!("tot={}, ({},{},{}), time:{:.2}s, speed:{:.6}T/d, left:{}h{}m{}s",
                    s.tot_cnt, s.max_sum, s.max_xp, s.max_xd, elapsed, speed,
                    time_left_secs / 3600, (time_left_secs % 3600) / 60, time_left_secs % 60);
            }
        }
    }
}

fn process_names(
    rng: &mut ChaCha20Rng,
    tmp: &mut [i8],
    prelen: usize,
    varlen: usize,
    scl: usize,
    charset_size: usize,
    charset: &[i8],
    l: u64,
    r: u64,
    task_type: u8,
    _name_len: usize,
    total_len_for_shuffle: usize,
    val_base: &[u8; N],
    model: &[f64; 1035],
    model_qd: &[f64; 1035],
    config: &ThreadConfig,
    xd_min: f64,
    output_file: &Arc<Mutex<BufWriter<File>>>,
    local_tot: &mut u64,
    local_max_sum: &mut i32,
    local_max_xp: &mut i32,
    local_max_xd: &mut i32,
) {
    let mut lastrnd: u64 = 0;
    let mut xp_min = config.xp_min;

    for i in l..r {
        // Enumerate variable part based on task type
        if task_type == 1 || task_type == 2 {
            let mut nn = i;
            for pos in (prelen..prelen + varlen * scl).rev().step_by(scl) {
                let cur = (nn % charset_size as u64) as usize;
                for ii in 0..scl {
                    tmp[pos + ii] = charset[cur * scl + ii];
                }
                nn /= charset_size as u64;
            }
        }

        if task_type == 3 {
            for pos in (prelen..prelen + varlen * scl).rev().step_by(scl) {
                if lastrnd <= charset_size as u64 { lastrnd = rng.gen(); }
                let cur = (lastrnd % charset_size as u64) as usize;
                for ii in 0..scl {
                    tmp[pos + ii] = charset[cur * scl + ii];
                }
                lastrnd /= charset_size as u64;
            }
        }

        // Build name_bytes from tmp (just the name part, with null terminator)
        let mut name_bytes: Vec<u8> = tmp[..total_len_for_shuffle].iter().map(|&b| b as u8).collect();
        name_bytes.push(0);

        // Compute val_base2 (team + prefix shuffle)
        let mut val_base2 = [0u8; N];
        {
            val_base2.copy_from_slice(val_base);
            let mut s: u8 = 0;
            let name_len = total_len_for_shuffle;
            let mut j = name_len;
            for ip in 0..prelen {
                let byte_at_j = if j < name_len { name_bytes[j] } else { 0 };
                s = s.wrapping_add(byte_at_j).wrapping_add(val_base2[ip]);
                val_base2.swap(ip, s as usize);
                j += 1;
                if j > name_len { j = 0; }
            }
        }

        let mut name_base = [0u8; 128];

        // Fast load with early pruning
        let v_opt = fast_load_and_check(
            &mut val_base2,
            prelen,
            total_len_for_shuffle,
            &name_bytes,
            &mut name_base,
        );

        let v = match v_opt {
            Some(v) => v,
            None => continue,
        };

        let props = compute_props(&name_base);

        // Full name length including suffix for calc_skills
        let full_name_bytes: Vec<u8> = tmp[..(total_len_for_shuffle + config.suffixes[0].len())]
            .iter().map(|&b| b as u8).collect();

        // First filter: V * 3 >= 1200 (standard)
        if v * 3 >= 1200 {
            process_name_v_high(
                &full_name_bytes,
                v as i32, &props,
                &val_base2, &name_bytes,
                &mut name_base,
                model, model_qd, xd_min, xp_min, config,
                output_file, local_tot, local_max_sum, local_max_xp, local_max_xd,
            );
        }
        // Second filter: V * 3 >= 1140 && prop[6] >= 36 && prop[1]+prop[2]+prop[5]+prop[6] >= 175
        else if v * 3 >= 1140
            && props[6] >= 36
            && props[1] + props[2] + props[5] + props[6] >= 175
        {
            process_name_v_medium(
                &full_name_bytes,
                v as i32, &props,
                &val_base2, &name_bytes,
                &mut name_base,
                model, model_qd, xd_min, xp_min, config,
                output_file, local_tot, local_max_sum, local_max_xp, local_max_xd,
            );
        }
    }
}

#[inline(always)]
fn fast_load_and_check(
    val_base2: &[u8; N],
    pre_len: usize,
    total_len: usize,
    name_bytes: &[u8],
    name_base: &mut [u8; 128],
) -> Option<u32> {
    let mut val = *val_base2;

    let total_len_p1 = total_len + 1;
    let mut s: u8 = 0;
    for i in pre_len..N {
        let j = (i - pre_len) % total_len_p1;
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }

    s = 0;
    for i in 0..N {
        let j = i % total_len_p1;
        s = s.wrapping_add(name_bytes[j]).wrapping_add(val[i]);
        val.swap(i, s as usize);
    }

    let mut q_len: isize = -1;

    for i in (0..96).step_by(8) {
        let u0 = val[i].wrapping_mul(181).wrapping_add(160);
        let u1 = val[i+1].wrapping_mul(181).wrapping_add(160);
        let u2 = val[i+2].wrapping_mul(181).wrapping_add(160);
        let u3 = val[i+3].wrapping_mul(181).wrapping_add(160);
        let u4 = val[i+4].wrapping_mul(181).wrapping_add(160);
        let u5 = val[i+5].wrapping_mul(181).wrapping_add(160);
        let u6 = val[i+6].wrapping_mul(181).wrapping_add(160);
        let u7 = val[i+7].wrapping_mul(181).wrapping_add(160);

        if u0 >= 89 && u0 < 217 { q_len += 1; name_base[q_len as usize] = u0 & 63; }
        if u1 >= 89 && u1 < 217 { q_len += 1; name_base[q_len as usize] = u1 & 63; }
        if u2 >= 89 && u2 < 217 { q_len += 1; name_base[q_len as usize] = u2 & 63; }
        if u3 >= 89 && u3 < 217 { q_len += 1; name_base[q_len as usize] = u3 & 63; }
        if u4 >= 89 && u4 < 217 { q_len += 1; name_base[q_len as usize] = u4 & 63; }
        if u5 >= 89 && u5 < 217 { q_len += 1; name_base[q_len as usize] = u5 & 63; }
        if u6 >= 89 && u6 < 217 { q_len += 1; name_base[q_len as usize] = u6 & 63; }
        if u7 >= 89 && u7 < 217 { q_len += 1; name_base[q_len as usize] = u7 & 63; }

        if q_len >= 30 { break; }
    }

    if q_len < 30 {
        for i in (96..N).step_by(8) {
            let u0 = val[i].wrapping_mul(181).wrapping_add(160);
            let u1 = val[i+1].wrapping_mul(181).wrapping_add(160);
            let u2 = val[i+2].wrapping_mul(181).wrapping_add(160);
            let u3 = val[i+3].wrapping_mul(181).wrapping_add(160);
            let u4 = val[i+4].wrapping_mul(181).wrapping_add(160);
            let u5 = val[i+5].wrapping_mul(181).wrapping_add(160);
            let u6 = val[i+6].wrapping_mul(181).wrapping_add(160);
            let u7 = val[i+7].wrapping_mul(181).wrapping_add(160);

            if u0 >= 89 && u0 < 217 { q_len += 1; name_base[q_len as usize] = u0 & 63; }
            if u1 >= 89 && u1 < 217 { q_len += 1; name_base[q_len as usize] = u1 & 63; }
            if u2 >= 89 && u2 < 217 { q_len += 1; name_base[q_len as usize] = u2 & 63; }
            if u3 >= 89 && u3 < 217 { q_len += 1; name_base[q_len as usize] = u3 & 63; }
            if u4 >= 89 && u4 < 217 { q_len += 1; name_base[q_len as usize] = u4 & 63; }
            if u5 >= 89 && u5 < 217 { q_len += 1; name_base[q_len as usize] = u5 & 63; }
            if u6 >= 89 && u6 < 217 { q_len += 1; name_base[q_len as usize] = u6 & 63; }
            if u7 >= 89 && u7 < 217 { q_len += 1; name_base[q_len as usize] = u7 & 63; }

            if q_len >= 30 { break; }
        }
    }

    if q_len < 30 { return None; }

    let mut v: u32 = 0;
    v += median3(name_base[28], name_base[29], name_base[30]) as u32;
    if v < 24 { return None; }
    v += median3(name_base[13], name_base[14], name_base[15]) as u32;
    v += median3(name_base[16], name_base[17], name_base[18]) as u32;
    v += median3(name_base[25], name_base[26], name_base[27]) as u32;
    if v < 165 { return None; }
    v += median3(name_base[10], name_base[11], name_base[12]) as u32;
    v += median3(name_base[19], name_base[20], name_base[21]) as u32;
    v += median3(name_base[22], name_base[23], name_base[24]) as u32;
    if v < 250 { return None; }

    let mut sorted10 = [0u8; 10];
    sorted10.copy_from_slice(&name_base[0..10]);
    sorted10.sort_unstable();
    v += (154u32 + sorted10[3] as u32 + sorted10[4] as u32 + sorted10[5] as u32 + sorted10[6] as u32) / 3;

    Some(v)
}

fn process_name_v_high(
    full_name_bytes: &[u8],
    v: i32,
    props: &[u32; 8],
    val_after_first_pass: &[u8; N],
    _name_bytes: &[u8],
    name_base: &mut [u8; 128],
    model: &[f64; 1035],
    model_qd: &[f64; 1035],
    xd_min: f64,
    mut xp_min: i32,
    config: &ThreadConfig,
    output_file: &Arc<Mutex<BufWriter<File>>>,
    local_tot: &mut u64,
    local_max_sum: &mut i32,
    local_max_xp: &mut i32,
    local_max_xd: &mut i32,
) {
    let name_len = full_name_bytes.len();
    let mut tmr_bytes = full_name_bytes.to_vec();
    if tmr_bytes.last() != Some(&0) { tmr_bytes.push(0); }
    let tmr_len = tmr_bytes.len() - 1;

    let mut val = *val_after_first_pass;
    fill_full_namebase(&val, name_base);
    calc_skills(&mut val, name_base, &mut [SkillSlot::default(); 16]);

    let mut flag = false;
    let mut flag3 = 0i32;

    let mut sum2: i32 = 0;
    let mut skills_out = [SkillSlot::default(); 16];
    calc_skills(&mut val, name_base, &mut skills_out);
    for s in 0..16 {
        if skills_out[s].id == 21 && skills_out[s].freq > 70 { flag = true; }
        if skills_out[s].id == 24 && skills_out[s].freq > 70 { flag = true; }
        if skills_out[s].id == 23 || skills_out[s].id == 11 || skills_out[s].id == 16 || skills_out[s].id == 28 {
            flag3 += skills_out[s].freq as i32;
        }
        sum2 += skills_out[s].freq as i32;
    }
    if flag3 >= 60 { flag = true; }

    let sum = v as i32 + 36 * 7;
    if sum2 >= 155 { flag = true; }
    if sum >= 697 { flag = true; }

    if config.collect_mode == 1 {
        if sum >= 777 { output_result(&tmr_bytes[..tmr_len], config, output_file); }
        if sum * 3 - props[7] as i32 >= 2000 { output_result(&tmr_bytes[..tmr_len], config, output_file); }
        if props[7] == 398 && sum >= 741 { output_result(&tmr_bytes[..tmr_len], config, output_file); }
        let hl = props.iter().take(7).min().unwrap() + 36;
        if hl >= 93 { output_result(&tmr_bytes[..tmr_len], config, output_file); }
    } else if config.collect_mode == 2 {
        if sum >= config.collect_8v_min { output_result(&tmr_bytes[..tmr_len], config, output_file); }
        if sum - props[7] as i32 / 3 >= config.collect_7v_min { output_result(&tmr_bytes[..tmr_len], config, output_file); }
        if props[7] == 398 && sum >= config.collect_hp8v_min { output_result(&tmr_bytes[..tmr_len], config, output_file); }
        let hl = *props.iter().take(7).min().unwrap() as i32 + 36;
        if hl >= config.collect_hl_min { output_result(&tmr_bytes[..tmr_len], config, output_file); }
    }

    if !flag { return; }

    let mut prop8 = [0u32; 8];
    for j in 0..7 { prop8[j] = props[j] + 36; }
    prop8[7] = props[7];

    let mut xp_x = [0.0f64; 44];
    xp_x[0] = prop8[7] as f64;
    for i in 0..7 { xp_x[i + 1] = prop8[i] as f64; }
    for i in 0..35 {
        xp_x[i + 8] = 0.0;
        for k in 0..16 {
            if skills_out[k].id as usize == i {
                xp_x[i + 8] = skills_out[k].freq as f64;
                break;
            }
        }
    }

    if xp_x[32] > 0.0 {
        let mut shadow_bytes = tmr_bytes.clone();
        shadow_bytes[tmr_len] = b'?';
        shadow_bytes[tmr_len + 1] = b's';
        shadow_bytes[tmr_len + 2] = b'h';
        shadow_bytes[tmr_len + 3] = b'a';
        shadow_bytes[tmr_len + 4] = b'd';
        shadow_bytes[tmr_len + 5] = b'o';
        shadow_bytes[tmr_len + 6] = b'w';
        shadow_bytes[tmr_len + 7] = 0;
        let shadow_len = tmr_len + 7;

        let mut shadow_val = [0u8; N];
        let mut shadow_val_base = [0u8; N];
        shadow_val_base.copy_from_slice(val_after_first_pass);
        {
            let t_len = shadow_len + 1;
            for i in 0..N { shadow_val_base[i] = i as u8; }
            let mut s: u8 = 0;
            for i in 0..N {
                if i % t_len != 0 { s = s.wrapping_add(shadow_bytes[i % t_len - 1]); }
                s = s.wrapping_add(shadow_val_base[i]);
                shadow_val_base.swap(i, s as usize);
            }
            let mut s: u8 = 0;
            for i in 0..N {
                if i % t_len != 0 { s = s.wrapping_add(shadow_bytes[i % t_len - 1]); }
                s = s.wrapping_add(shadow_val_base[i]);
                shadow_val_base.swap(i, s as usize);
            }
        }

        let mut shadow_nb = [0u8; 128];
        fill_full_namebase(&shadow_val_base, &mut shadow_nb);

        let s_prop_0 = median3(shadow_nb[10], shadow_nb[11], shadow_nb[12]) as i32;
        let s_prop_1 = median3(shadow_nb[13], shadow_nb[14], shadow_nb[15]) as i32;
        let s_prop_2 = median3(shadow_nb[16], shadow_nb[17], shadow_nb[18]) as i32;
        let s_prop_3 = median3(shadow_nb[19], shadow_nb[20], shadow_nb[21]) as i32;
        let s_prop_4 = median3(shadow_nb[22], shadow_nb[23], shadow_nb[24]) as i32;
        let s_prop_5 = median3(shadow_nb[25], shadow_nb[26], shadow_nb[27]) as i32;
        let s_prop_6 = median3(shadow_nb[28], shadow_nb[29], shadow_nb[30]) as i32;
        let s_prop_7 = 154 + shadow_nb[3] as i32 + shadow_nb[4] as i32 + shadow_nb[5] as i32 + shadow_nb[6] as i32;

        let mut shadow_sum = s_prop_7 as f64 / 3.0;
        for j in 0..7 { shadow_sum += [s_prop_0, s_prop_1, s_prop_2, s_prop_3, s_prop_4, s_prop_5, s_prop_6][j] as f64; }
        shadow_sum -= s_prop_6 as f64 * 3.0;
        let shadowi = (shadow_sum - 210.0) * xp_x[32] / 100.0;
        xp_x[43] = shadowi;
    } else {
        xp_x[43] = 0.0;
    }

    if xp_x[42] > 0.0 { xp_x[42] += 20.0; }

    let mut xp_array = [0.0f64; 1034];
    hanxu_poly(&mut xp_array, &xp_x);

    let mut score = model[0];
    let mut score_qd = model_qd[0];
    for i in 0..1034 {
        score += xp_array[i] * model[i + 1];
    }

    if score < 4300.0 {
        score_qd = 0.0;
    } else {
        for i in 0..1034 {
            score_qd += xp_array[i] * model_qd[i + 1];
        }
    }

    if flag3 >= 50 { xp_min -= 300; }
    if score >= xp_min as f64 || score_qd > xd_min {
        output_result_with_score(&tmr_bytes[..tmr_len], score as i32, score_qd as i32, config, output_file);
        *local_tot += 1;
        let sum = v + 36 * 7;
        if sum as i32 > *local_max_sum { *local_max_sum = sum as i32; }
        if score as i32 > *local_max_xp { *local_max_xp = score as i32; }
        if score_qd as i32 > *local_max_xd { *local_max_xd = score_qd as i32; }
    }
    if flag3 >= 50 { xp_min += 300; }
}

fn process_name_v_medium(
    full_name_bytes: &[u8],
    v: i32,
    props: &[u32; 8],
    val_after_first_pass: &[u8; N],
    name_bytes: &[u8],
    name_base: &mut [u8; 128],
    model: &[f64; 1035],
    model_qd: &[f64; 1035],
    xd_min: f64,
    xp_min: i32,
    config: &ThreadConfig,
    output_file: &Arc<Mutex<BufWriter<File>>>,
    local_tot: &mut u64,
    local_max_sum: &mut i32,
    local_max_xp: &mut i32,
    local_max_xd: &mut i32,
) {
    process_name_v_high(
        full_name_bytes, v, props, val_after_first_pass, name_bytes, name_base,
        model, model_qd, xd_min, xp_min, config, output_file,
        local_tot, local_max_sum, local_max_xp, local_max_xd,
    );
}

fn output_result(name_bytes: &[u8], config: &ThreadConfig, output_file: &Arc<Mutex<BufWriter<File>>>) {
    let name_str = String::from_utf8_lossy(name_bytes);
    let team_vec = config.team_name.iter().filter(|&&b| b != 0).map(|&b| b as u8).collect::<Vec<_>>();
    let team_str = String::from_utf8_lossy(&team_vec);
    let mut f = output_file.lock().unwrap();
    let _ = writeln!(f, "{}@{}", name_str, team_str);
    let _ = f.flush();
}

fn output_result_with_score(name_bytes: &[u8], xp: i32, xd: i32, config: &ThreadConfig, output_file: &Arc<Mutex<BufWriter<File>>>) {
    let name_str = String::from_utf8_lossy(name_bytes);
    let team_vec = config.team_name.iter().filter(|&&b| b != 0).map(|&b| b as u8).collect::<Vec<_>>();
    let team_str = String::from_utf8_lossy(&team_vec);
    let mut f = output_file.lock().unwrap();
    if config.output_xp {
        let _ = writeln!(f, "{}@{} {} {}", name_str, team_str, xp, xd);
    } else {
        let _ = writeln!(f, "{}@{}", name_str, team_str);
    }
    let _ = f.flush();
}
