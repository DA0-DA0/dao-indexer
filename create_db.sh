#!/bin/sh
# create a db in the local "data" dir
mkdir ./data
pg_ctl init -D data
pg_ctl -D data -l logfile start
