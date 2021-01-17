-- Add migration script here

insert into trade_record select time, 'ETH_BTC', 1, random()*0.1, random()*100 
from generate_series(timestamp '2020-01-02 00:00:00', timestamp '2020-01-03 00:00:00', interval '1 s') as time;

insert into trade_record select time, 'ETH_BTC', 2, random()*0.1, random()*100 
from generate_series(timestamp '2020-01-02 00:00:00', timestamp '2020-01-03 00:00:00', interval '2 s') as time;

insert into trade_record select time, 'ETH_BTC', 3, random()*0.1, random()*100 
from generate_series(timestamp '2020-01-02 00:00:00', timestamp '2020-01-03 00:00:00', interval '5 s') as time;