DB="postgres://exchange:exchange_AA9944@127.0.0.1/exchange"
DB_RESET_DIR="migrations/reset"
BUILD_MODE="debug"
PROCESSES="restapi|persistor|matchengine"

fmtproto:
	clang-format -i proto/exchange/matchengine.proto

fmtsql:
	find migrations -type f | xargs -L 1 pg_format --type-case 2 -i

fmtrs:
	cargo fmt

fmtjs:
	cd examples/js && yarn fmt

fmt: fmtproto fmtsql fmtrs fmtjs

startall:
	cargo build
	mkdir -p logs
	./target/$(BUILD_MODE)/matchengine 1>logs/matchengine.log 2>&1 &
	./target/$(BUILD_MODE)/persistor 1>logs/persistor.log 2>&1 &
	./target/$(BUILD_MODE)/restapi 1>logs/restapi.log 2>&1 &
pgrep:
	pgrep -l $(PROCESSES) || true


viewlogs:
	watch -n 0.5 tail -n 5 logs/*

stopall:
	pkill -INT $(PROCESSES) || true
	(pgrep -l $(PROCESSES) && (echo '"pkill -INT" failed, force kill'; pkill -l $(PROCESSES))) || true

conn:
	psql $(DB)

cleardb:
	# https://stackoverflow.com/a/13823560/2078461
	psql $(DB) -X -a -f $(DB_RESET_DIR)/down.sql
	psql $(DB) -X -a -f $(DB_RESET_DIR)/up.sql
