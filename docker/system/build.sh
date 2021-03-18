#!/bin/bash
# Copyright 2016 Google Inc. All Rights Reserved.
# Licensed under the Apache License, Version 2.0 (the "License");
. $(dirname $0)/common.sh
BUILD=$SCRIPT_DIR/$1/build.sh

[ ! -e $BUILD ] && echo "NO SUCH FILE: $BUILD" && exit 1

RUNDIR="RUNDIR-$1"
mkdir -p $RUNDIR
cd $RUNDIR
$BUILD
