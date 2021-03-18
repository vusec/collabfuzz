FROM fuzzer-system:afl

WORKDIR /work
# install AFL fairfuzz
RUN git clone https://github.com/carolemieux/afl-rb.git && cd afl-rb && git checkout e529c1f1b366 && cp -r ../afl/llvm_mode ./ && make -j`nproc` && cd llvm_mode && make -j`nproc` && cd ../ && ls && ./afl-clang --version && make install

FROM fuzzer-afl
USER root
COPY --from=0 /work /work
WORKDIR /work/afl
RUN make install
RUN rm -rf /work

USER coll
WORKDIR /home/coll
