CREATE TABLE
  IF NOT EXISTS quote (
    symbol String,
    ask_exchange_code String,
    ask_price Float64,
    ask_size UInt64,
    bid_exchange_code String,
    bid_price Float64,
    bid_size UInt64,
    condition Array (String),
    timestamp DateTime64 (9, 'UTC'),
    tape String
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
