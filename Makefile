DB="postgres://exchange:exchange_AA9944@127.0.0.1/exchange"
RESET_CMD="DROP SCHEMA public CASCADE; CREATE SCHEMA public; GRANT ALL ON SCHEMA public TO public;"

fmtproto:
	clang-format -i proto/exchange/matchengine.proto

fmtsql:
	find migrations -type f | xargs -L 1 pg_format --type-case 2 -i

fmtrs:
	cargo fmt

fmt: fmtproto fmtsql fmtrs

conn:
	psql $(DB)

cleardb:
	# https://stackoverflow.com/a/13823560/2078461
	psql $(DB) -c $(RESET_CMD)
