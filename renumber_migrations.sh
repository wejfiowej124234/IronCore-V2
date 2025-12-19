#!/bin/bash
set -e

cd "$(dirname "$0")/migrations"

echo "开始重新编号迁移文件..."

# 获取所有.sql文件并按当前版本号排序
files=($(ls -1 *.sql | sort))

# 计数器从1开始
counter=1

# 临时目录
mkdir -p ../migrations_renumbered

for file in "${files[@]}"; do
    # 提取描述部分（去掉版本号前缀）
    description=$(echo "$file" | sed 's/^[0-9]\{4\}_//')
    
    # 新文件名（版本号补齐为4位）
    new_name=$(printf "%04d_%s" $counter "$description")
    
    echo "$file -> $new_name"
    
    # 复制到新目录
    cp "$file" "../migrations_renumbered/$new_name"
    
    counter=$((counter + 1))
done

echo ""
echo "重新编号完成！共 $((counter - 1)) 个迁移文件"
echo "新文件在: migrations_renumbered/"
echo ""
echo "验证新文件："
ls -1 ../migrations_renumbered/*.sql | head -10
