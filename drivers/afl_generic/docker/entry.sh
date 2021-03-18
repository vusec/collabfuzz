#!/usr/bin/env bash

DOCKER_SOCK_PATH='/var/run/docker.sock'
DRIVER_USER='coll'

set -euo pipefail

if [[ ! -S "$DOCKER_SOCK_PATH" ]]
then
	echo "Docker socket not found: $DOCKER_SOCK_PATH"
	exit 1
fi

echo "Docker socket found: $DOCKER_SOCK_PATH"

read -r -a LS_OUTPUT <<< "$(ls -l --numeric-uid-gid "$DOCKER_SOCK_PATH")"
DOCKER_GID=${LS_OUTPUT[3]}

echo "Docker GID: $DOCKER_GID"

addgroup --system --gid "$DOCKER_GID" docker
adduser "$DRIVER_USER" docker

echo "Preparing signal handler"

term_handler() {
	echo 'SIGTERM received!'

	driver_pid=$(pgrep collabfuzz)
	if [[ $driver_pid != "" ]]
	then
		exit_code=0
		while [[ $exit_code == 0 ]]
		do
			kill -SIGTERM "$driver_pid" 2> /dev/null || \
				exit_code=${?}
			sleep 0.1
		done
	fi

	exit 143
}

trap 'kill ${!}; term_handler' SIGTERM

echo "Starting driver as $DRIVER_USER..."

if [[ -n "$ARG" ]]
then
	OPT_ARGS="-- $ARG"
else
	OPT_ARGS=''
fi

runuser -u "$DRIVER_USER" -- \
	collabfuzz_generic_driver \
		--verbose \
		--enable-docker \
		--afl-path "$AFL_PATH" \
		"$FUZZER_NAME" \
		"$OUTPUT_DIR" \
		$OPT_ARGS \
	&

while true
do
	tail -f /dev/null & wait ${!}
done
