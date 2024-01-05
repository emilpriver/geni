#!/bin/bash

mkdir migrations
cd migrations
# Create two SQL files with CREATE TABLE statements
for i in {1..2}; do
	timestamp=$(date +%Y%m%d%H%M%S)
	filename_up="${timestamp}${i}_test_${i}.up.sql"

	echo "CREATE TABLE table_${i} (
      id INT PRIMARY KEY,
      name VARCHAR(255) NOT NULL,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    );" >$filename_up

	filename_down="${timestamp}${i}_test_${i}.down.sql"

	echo "DROP TABLE table_${i};" >$filename_down
done
