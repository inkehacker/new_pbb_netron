#!/bin/bash
#SBATCH -J pbb_32cores
#SBATCH -N 1
#SBATCH --ntasks-per-node=1      # 单进程
#SBATCH --cpus-per-task=32       # 分配 32 个 CPU 核心给该进程
#SBATCH -p kshctest
#SBATCH --time=72:00:00          # 10 分钟，可根据需要调整
#SBATCH --exclusive              # 独占节点，确保获得全部 32 核

# 加载编译环境（与编译时一致）
module load compiler/gcc/12.2.0	

# 设置 OpenMP 线程数（如果你的程序使用 OpenMP）
export OMP_NUM_THREADS=32

# 运行程序
./pbb_kunshan32 < input.txt