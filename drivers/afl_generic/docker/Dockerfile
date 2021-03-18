FROM python:3 AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends protobuf-compiler && \
    pip install pipenv

WORKDIR /build

# The Pipfile installs the current package as editable, remove it from the
# requirements.
COPY Pipfile Pipfile.lock setup.py ./
RUN ["/bin/bash", "-c", " \
    set -o pipefail && \
    pipenv lock --requirements --keep-outdated \
        | sed '/-e ./d' > requirements.txt \
    "]

# A separate build step is necessary since the *_pb2.py files need to be
# generated separately
COPY . ./
RUN python setup.py bdist_wheel

FROM python:3 AS production

RUN adduser --disabled-password --gecos "" coll && \
    mkdir /data && \
    chown -R coll:coll /data

ENV FUZZER_NAME=afl OUTPUT_DIR=/out AFL_PATH=/fuzzers/afl ARG=''

WORKDIR /app

COPY --from=builder /build/requirements.txt .
RUN pip install --requirement requirements.txt

COPY --from=builder /build/dist/collabfuzz_generic_driver-*.whl .
RUN pip install collabfuzz_generic_driver-*.whl

COPY docker/entry.sh .

CMD ["./entry.sh"]
