# lg-rs

Command line utility for creating log groupings.

This is built on [drain-rs](https://github.com/benwtrent/drain-rs/)

Usage:

Creating a new model
```
lg-rs HDFS_2k.log --log-pattern="%{NUMBER:date} %{NUMBER:time} %{NUMBER:proc} %{LOGLEVEL:level} %{DATA:component}: %{GREEDYDATA:content}" --filter-patterns="blk_(|-)[0-9]+,%{IPV4:ip_address},%{NUMBER:number}" --output-model=dump.json
```

Reading from an existing one
```
cat HDFS_2k.log | lg-rs --from-model=dump.json
```
