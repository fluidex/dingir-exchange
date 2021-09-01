DB="postgres://exchange:exchange_AA9944@127.0.0.1/exchange"
DB_RESET_DIR="migrations/reset"
BUILD_MODE="debug"
# tokio-runtime-worker does not work well, do not know why...
PROCESSES="openapi|restapi|persistor|matchengine|tokio-runtime-w|ticker.ts"
CURRENTDATE=`date +"%Y-%m-%d"`

# code related
lint:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
fmtsql:
	find migrations -type f | xargs -L 1 pg_format --type-case 2 -i
fmtrs:
	cargo fmt --all
fmtjs:
	cd examples/js && yarn fmt
fmt: fmtsql fmtrs fmtjs

# docker related
start-compose:
	cd orchestra/docker; docker compose up -d exchange_db exchange_zookeeper exchange_kafka exchange_envoy
stop-compose:
	cd orchestra/docker; docker compose down exchange_db exchange_zookeeper exchange_kafka exchange_envoy
clean-compose: stop-compose 
	rm -rf orchestra/docker/volumes/exchange_*

# process relared
startall:
	cargo build
	mkdir -p logs
	`pwd`/target/$(BUILD_MODE)/matchengine 1>logs/matchengine.$(CURRENTDATE).log 2>&1 &
	# fix the migrator order problem
	sleep 3; `pwd`/target/$(BUILD_MODE)/persistor 1>logs/persistor.$(CURRENTDATE).log 2>&1 &
	`pwd`/target/$(BUILD_MODE)/openapi 1>logs/openapi.$(CURRENTDATE).log 2>&1 &
	`pwd`/target/$(BUILD_MODE)/restapi 1>logs/restapi.$(CURRENTDATE).log 2>&1 &
list:
	pgrep -l $(PROCESSES) || true
stopall:
	pkill -INT $(PROCESSES) || true
	(pgrep -l $(PROCESSES) && (echo '"pkill -INT" failed, force kill'; pkill $(PROCESSES))) || true

# logs related
taillogs:
	tail -n 15 logs/*
viewlogs:
	watch -n 0.5 tail -n 4 logs/*
rmlogs:
	rm -rf logs/*


# db related
conn:
	psql $(DB)
cleardb:
	# https://stackoverflow.com/a/13823560/2078461
	psql $(DB) -X -a -f $(DB_RESET_DIR)/down.sql
	psql $(DB) -X -a -f $(DB_RESET_DIR)/up.sql
dump-trades:
	psql $(DB) -c "select count(*) from user_trade where market = 'ETH_USDT' and user_id = 6"
	psql $(DB) -t -A -F"," -c "select time, side, role, price, amount, quote_amount from user_trade where market = 'ETH_USDT' and user_id = 6 order by time asc" > trades.csv
