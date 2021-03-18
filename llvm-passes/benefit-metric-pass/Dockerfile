FROM archlinux/base

RUN pacman -Syu --noconfirm base base-devel curl flex bison libtool make automake autoconf gcc grep git python-pipenv cmake llvm llvm-libs boost clucene clang openssl gdb rsync tar the_silver_searcher watchexec

WORKDIR /work

ENTRYPOINT ["watchexec", "./run.sh"]
