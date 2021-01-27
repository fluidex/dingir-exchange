-- Add migration script here

insert into trade_record select time, 'ETH_USDT', 1, price, amount, price * amount, 'ask'
from (select time, random()*300 + 1000 as price, random()*10 as amount 
from generate_series(now() - interval '2 day', now(), interval '1 s') as time) t;

insert into trade_record select time, 'ETH_USDT', 1, price, amount, price * amount, 'ask'
from (select time, random()*200 + 1000 as price, random()*30 as amount 
from generate_series(now() - interval '2 day', now(), interval '3 s') as time) t;

insert into trade_record select time, 'ETH_USDT', 1, price, amount, price * amount, 'ask'
from (select time, random()*200 + 1000 as price, random()*100 as amount 
from generate_series(now() - interval '2 day', now(), interval '15 s') as time) t;
