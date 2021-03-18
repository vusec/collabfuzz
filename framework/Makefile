.PHONY: docker proj

all: docker

proj:
	cargo build --release

docker: 
	docker build -t fuzzer-framework ../
