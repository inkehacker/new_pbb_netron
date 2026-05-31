use std::io::{self, Write, BufRead};

use rust_pbb::miner::{MinerConfig, run_miner};
use rust_pbb::namecalc::load_team;
use rust_pbb::charset;

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("Rust pbb测号器");
    eprintln!("使用前请确保当前目录下有 out/ 文件夹");

    eprintln!("请输入队名：");
    stdout.flush().unwrap();
    let mut team_input = String::new();
    stdin.lock().read_line(&mut team_input).unwrap();
    let team_input = team_input.trim();

    let team_bytes: Vec<i8>;
    if team_input.starts_with('@') {
        eprintln!("输入转化编码后的长度：");
        let mut len_str = String::new();
        stdin.lock().read_line(&mut len_str).unwrap();
        let team_name_len: usize = len_str.trim().parse().unwrap_or(0);

        eprintln!("输入转化编码后的每个数字：");
        let mut num_str = String::new();
        stdin.lock().read_line(&mut num_str).unwrap();
        let mut vals = Vec::new();
        for s in num_str.split_whitespace() {
            if let Ok(v) = s.parse::<i8>() {
                vals.push(v);
            }
        }
        vals.resize(team_name_len, 0);
        team_bytes = vals;
    } else {
        team_bytes = team_input.as_bytes().iter().map(|&b| b as i8).collect();
    }

    let team_display = String::from_utf8_lossy(
        &team_bytes.iter().filter(|&&b| b != 0).map(|&b| b as u8).collect::<Vec<_>>()
    ).to_string();

    eprintln!("队名：{}", team_display);

    eprintln!("输入前缀数量（无前缀也要输入 1 ）：");
    stdout.flush().unwrap();
    let mut prefix_count_str = String::new();
    stdin.lock().read_line(&mut prefix_count_str).unwrap();
    let prefix_count: usize = prefix_count_str.trim().parse().unwrap_or(1);

    let mut prefixes: Vec<Vec<i8>> = Vec::new();
    let mut prefix_lens: Vec<usize> = Vec::new();
    for i in 1..=prefix_count {
        eprintln!("输入第 {} 个前缀（如果无前缀输入一个单独的加号）：", i);
        stdout.flush().unwrap();
        let mut prefix_input = String::new();
        stdin.lock().read_line(&mut prefix_input).unwrap();
        let prefix_input = prefix_input.trim();

        if prefix_input.starts_with('@') {
            eprintln!("输入转化编码后的长度：");
            let mut len_str = String::new();
            stdin.lock().read_line(&mut len_str).unwrap();
            let len: usize = len_str.trim().parse().unwrap_or(0);
            eprintln!("输入转化编码后的每个数字：");
            let mut num_str = String::new();
            stdin.lock().read_line(&mut num_str).unwrap();
            let mut vals = Vec::new();
            for s in num_str.split_whitespace() {
                if let Ok(v) = s.parse::<i8>() { vals.push(v); }
            }
            vals.resize(len, 0);
            prefix_lens.push(len);
            prefixes.push(vals);
        } else if prefix_input == "+" {
            prefix_lens.push(0);
            prefixes.push(vec![]);
        } else {
            let bytes: Vec<i8> = prefix_input.as_bytes().iter().map(|&b| b as i8).collect();
            prefix_lens.push(bytes.len());
            prefixes.push(bytes);
        }
    }

    eprintln!("输入后缀数量（无后缀也要输入 1 ）：");
    stdout.flush().unwrap();
    let mut suffix_count_str = String::new();
    stdin.lock().read_line(&mut suffix_count_str).unwrap();
    let suffix_count: usize = suffix_count_str.trim().parse().unwrap_or(1);

    let mut suffixes: Vec<Vec<i8>> = Vec::new();
    let mut suffix_lens: Vec<usize> = Vec::new();
    for i in 1..=suffix_count {
        eprintln!("输入第 {} 个后缀（如果无后缀输入一个单独的加号）：", i);
        stdout.flush().unwrap();
        let mut suffix_input = String::new();
        stdin.lock().read_line(&mut suffix_input).unwrap();
        let suffix_input = suffix_input.trim();

        if suffix_input.starts_with('@') {
            eprintln!("输入转化编码后的长度：");
            let mut len_str = String::new();
            stdin.lock().read_line(&mut len_str).unwrap();
            let len: usize = len_str.trim().parse().unwrap_or(0);
            eprintln!("输入转化编码后的每个数字：");
            let mut num_str = String::new();
            stdin.lock().read_line(&mut num_str).unwrap();
            let mut vals = Vec::new();
            for s in num_str.split_whitespace() {
                if let Ok(v) = s.parse::<i8>() { vals.push(v); }
            }
            vals.resize(len, 0);
            suffix_lens.push(len);
            suffixes.push(vals);
        } else if suffix_input == "+" {
            suffix_lens.push(0);
            suffixes.push(vec![]);
        } else {
            let bytes: Vec<i8> = suffix_input.as_bytes().iter().map(|&b| b as i8).collect();
            suffix_lens.push(bytes.len());
            suffixes.push(bytes);
        }
    }

    let charset_config = choose_charset();
    let charset = charset_config.charset;
    let charset_size = charset_config.charset_size;
    let single_char_len = charset_config.single_char_len;

    eprintln!("请选择枚举模式：");
    eprintln!("1. 顺序");
    eprintln!("2. 随机（随机若干个长度为 1e6 的区间进行顺序）");
    eprintln!("3. 随机（按位随机）");
    eprintln!("4. 随机（前缀和后缀一一对应）");
    stdout.flush().unwrap();
    let mut tp_str = String::new();
    stdin.lock().read_line(&mut tp_str).unwrap();
    let task_type: u8 = tp_str.trim().parse().unwrap_or(2);

    let variable_len: usize;
    let sequential_ranges: Vec<(u64, u64)>;
    let random_total: u64;

    if task_type == 1 {
        eprintln!("输入可变长度：");
        stdout.flush().unwrap();
        let mut vl_str = String::new();
        stdin.lock().read_line(&mut vl_str).unwrap();
        variable_len = vl_str.trim().parse().unwrap_or(1);

        eprintln!("输入区间组的数量：");
        stdout.flush().unwrap();
        let mut rn_str = String::new();
        stdin.lock().read_line(&mut rn_str).unwrap();
        let tot_range_number: usize = rn_str.trim().parse().unwrap_or(1);

        let mut all_ranges = Vec::new();
        for i in 1..=tot_range_number {
            eprintln!("输入第 {} 个区间组 (n,[l,r))：", i);
            stdout.flush().unwrap();
            let mut range_str = String::new();
            stdin.lock().read_line(&mut range_str).unwrap();
            let parts: Vec<u64> = range_str.split_whitespace()
                .filter_map(|s| s.parse().ok()).collect();
            if parts.len() >= 3 {
                for _ in 0..parts[0] { all_ranges.push((parts[1], parts[2])); }
            }
        }
        sequential_ranges = all_ranges;
        random_total = 0;
    } else {
        eprintln!("输入可变长度：");
        stdout.flush().unwrap();
        let mut vl_str = String::new();
        stdin.lock().read_line(&mut vl_str).unwrap();
        variable_len = vl_str.trim().parse().unwrap_or(1);

        eprintln!("输入总数（如果输入 -1 表示 1e18）：");
        stdout.flush().unwrap();
        let mut rt_str = String::new();
        stdin.lock().read_line(&mut rt_str).unwrap();
        let rt: i64 = rt_str.trim().parse().unwrap_or(-1);
        random_total = if rt == -1 { 1_000_000_000_000_000_000 } else { rt as u64 };
        sequential_ranges = Vec::new();
    }

    eprintln!("输入虚评阈值和虚单阈值：");
    stdout.flush().unwrap();
    let mut thresholds_str = String::new();
    stdin.lock().read_line(&mut thresholds_str).unwrap();
    let thresholds: Vec<i32> = thresholds_str.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    let xp_min = thresholds.get(0).copied().unwrap_or(4900);
    let xd_min = thresholds.get(1).copied().unwrap_or(5700);

    eprintln!("是否收集特殊属性号（0=否，1=破纪录，2=自定义阈值）：");
    stdout.flush().unwrap();
    let mut collect_str = String::new();
    stdin.lock().read_line(&mut collect_str).unwrap();
    let collect_mode: u8 = collect_str.trim().parse().unwrap_or(0);

    let mut collect_8v_min = 0i32;
    let mut collect_7v_min = 0i32;
    let mut collect_hl_min = 0i32;
    let mut collect_hp8v_min = 0i32;
    if collect_mode == 2 {
        eprintln!("按顺序输入八围阈值、七围阈值、华丽阈值、HP=398 的八围阈值：");
        stdout.flush().unwrap();
        let mut collect_str2 = String::new();
        stdin.lock().read_line(&mut collect_str2).unwrap();
        let cv: Vec<i32> = collect_str2.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        collect_8v_min = cv.get(0).copied().unwrap_or(700);
        collect_7v_min = cv.get(1).copied().unwrap_or(2000);
        collect_hl_min = cv.get(2).copied().unwrap_or(93);
        collect_hp8v_min = cv.get(3).copied().unwrap_or(741);
    }

    eprintln!("是否在输出文件中显示虚评虚单（0=否，1=是）：");
    stdout.flush().unwrap();
    let mut oxp_str = String::new();
    stdin.lock().read_line(&mut oxp_str).unwrap();
    let output_xp: bool = oxp_str.trim() == "1";

    eprintln!("是否将 UTF-8 编码直接输出（0=转换，1=直接输出）：");
    stdout.flush().unwrap();
    let mut utf_str = String::new();
    stdin.lock().read_line(&mut utf_str).unwrap();
    let output_utf: bool = utf_str.trim() == "1";

    eprintln!("输出文件的文件名（输入 + 生成随机，输入 - 生成时间戳）：");
    stdout.flush().unwrap();
    let mut fname_str = String::new();
    stdin.lock().read_line(&mut fname_str).unwrap();
    let fname_str = fname_str.trim();

    let output_file = if fname_str == "+" {
        let rn: String = (0..7).map(|_| (rand::random::<u8>() % 10 + b'0') as char).collect();
        format!("./out/{}.txt", rn)
    } else if fname_str == "-" {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        format!("./out/out-{}.txt", ts)
    } else {
        format!("./out/{}.txt", fname_str)
    };

    eprintln!("输入你要开的线程数：");
    stdout.flush().unwrap();
    let mut nt_str = String::new();
    stdin.lock().read_line(&mut nt_str).unwrap();
    let num_threads: usize = nt_str.trim().parse().unwrap_or(num_cpus::get());

    eprintln!("开始测号...");

    let config = MinerConfig {
        team_name: team_bytes,
        team_name_display: team_display,
        prefixes,
        prefix_lens,
        suffixes,
        suffix_lens,
        charset,
        charset_size,
        single_char_len,
        variable_len,
        task_type,
        sequential_ranges,
        random_total,
        xp_min,
        xd_min,
        collect_mode,
        collect_8v_min,
        collect_7v_min,
        collect_hl_min,
        collect_hp8v_min,
        output_xp,
        output_utf,
        num_threads,
        output_file,
    };

    std::fs::create_dir_all("./out").ok();
    run_miner(&config);
}

struct CharsetConfig {
    charset: Vec<i8>,
    charset_size: usize,
    single_char_len: usize,
}

fn choose_charset() -> CharsetConfig {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    eprintln!("请输入字符集中单个字符的长度：");
    eprintln!("1=ASCII, 2=希腊/俄文/拉丁, 3=汉字/假名/片假名/盲文, 4=扩展汉字");
    stdout.flush().unwrap();
    let mut scl_str = String::new();
    stdin.lock().read_line(&mut scl_str).unwrap();
    let single_char_len: usize = scl_str.trim().parse().unwrap_or(3);

    let mut charset: Vec<i8> = Vec::new();

    match single_char_len {
        1 => {
            eprintln!("1. 数字（10个） 2. 小写字母（26个） 3. 大写字母（26个）");
            eprintln!("请输入选择的编号（逗号分隔）：");
            stdout.flush().unwrap();
            let mut sel_str = String::new();
            stdin.lock().read_line(&mut sel_str).unwrap();
            for s in sel_str.split(|c: char| !c.is_ascii_digit()) {
                if s.is_empty() { continue; }
                match s {
                    "1" => { for i in 0..10u8 { charset.push((b'0' + i) as i8); } eprintln!("数字已加入"); }
                    "2" => { for i in 0..26u8 { charset.push((b'a' + i) as i8); } eprintln!("小写字母已加入"); }
                    "3" => { for i in 0..26u8 { charset.push((b'A' + i) as i8); } eprintln!("大写字母已加入"); }
                    _ => {}
                }
            }
        }
        2 => {
            eprintln!("1. 小写希腊 2. 大写希腊 3. 小写俄文 4. 大写俄文 5. 小写拉丁 6. 大写拉丁");
            eprintln!("请输入选择的编号（逗号分隔）：");
            stdout.flush().unwrap();
            let mut sel_str = String::new();
            stdin.lock().read_line(&mut sel_str).unwrap();
            for s in sel_str.split(|c: char| !c.is_ascii_digit()) {
                if s.is_empty() { continue; }
                match s {
                    "1" => { charset.extend_from_slice(&charset::XILA_LOWER); eprintln!("小写希腊已加入"); }
                    "2" => { charset.extend_from_slice(&charset::XILA_UPPER); eprintln!("大写希腊已加入"); }
                    "3" => { charset.extend_from_slice(&charset::EWEN_LOWER); eprintln!("小写俄文已加入"); }
                    "4" => { charset.extend_from_slice(&charset::EWEN_UPPER); eprintln!("大写俄文已加入"); }
                    "5" => { charset.extend_from_slice(&charset::LADING_LOWER); eprintln!("小写拉丁已加入"); }
                    "6" => { charset.extend_from_slice(&charset::LADING_UPPER); eprintln!("大写拉丁已加入"); }
                    _ => {}
                }
            }
        }
        3 => {
            eprintln!("1. 基本汉字 2. 常用汉字 3. 平假名 4. 片假名 5. 盲文 6. 扩展汉字(3)");
            eprintln!("请输入选择的编号（逗号分隔）：");
            stdout.flush().unwrap();
            let mut sel_str = String::new();
            stdin.lock().read_line(&mut sel_str).unwrap();
            for s in sel_str.split(|c: char| !c.is_ascii_digit()) {
                if s.is_empty() { continue; }
                match s {
                    "1" => { let h = charset::load_hanzi(19968, 40959); charset.extend_from_slice(&h); eprintln!("基本汉字已加入"); }
                    "2" => { charset.extend_from_slice(&charset::HANZI_COMMON); eprintln!("常用汉字已加入"); }
                    "3" => { charset.extend_from_slice(&charset::HIRAGANA); eprintln!("平假名已加入"); }
                    "4" => { charset.extend_from_slice(&charset::KATAKANA); eprintln!("片假名已加入"); }
                    "5" => { charset.extend_from_slice(&charset::BRAILLE); eprintln!("盲文已加入"); }
                    "6" => { let h = charset::load_exhanzi(13312, 19711); charset.extend_from_slice(&h); eprintln!("扩展汉字(3)已加入"); }
                    _ => {}
                }
            }
        }
        4 => {
            eprintln!("1. 扩展汉字（长度4）");
            stdout.flush().unwrap();
            let mut sel_str = String::new();
            stdin.lock().read_line(&mut sel_str).unwrap();
            if sel_str.trim() == "1" {
                let h = charset::load_exhanzi(131072, 175999);
                charset.extend_from_slice(&h);
                eprintln!("扩展汉字(4)已加入");
            }
        }
        _ => { eprintln!("不支持的字符长度"); }
    }

    let charset_size = charset.len() / single_char_len;
    eprintln!("字符集大小: {}", charset_size);

    CharsetConfig { charset, charset_size, single_char_len }
}
