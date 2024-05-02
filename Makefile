CC = cargo
EXE = Crab

.PHONY: all help clean

default: all

all: $(EXE)

help:
	@echo "This is a wrapper over Cargo, for OpenBench. Type:"
	@echo ""
	@echo "make [target]"
	@echo ""
	@echo "Where target is one of the following:"
	@echo ""
	@echo "all      > Runs 'cargo build --release' and copies the binary to"
	@echo "           the root directory"
	@echo "help:    > Print this message"
	@echo "clean:   > Removes the binary generated with 'make'. Does not run"
	@echo "           'cargo clean'."
	@echo ""
	@echo "If no target is given, it will use \"all\""

clean:
	rm -f $(EXE)

$(EXE):
	$(CC) build --release
	cp target/release/crab $@
