#!/bin/sh
psql postgres://gavin@localhost:5432/postgres -c "drop database schemaindexer;" -c "create database schemaindexer;"
