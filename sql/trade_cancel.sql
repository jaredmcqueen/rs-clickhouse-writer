CREATE TABLE
  IF NOT EXISTS trade_cancel (
    symbol String,
    id UInt64,
    exchange_code String,
    price Float64,
    size UInt64,
    action String,
    timestamp DateTime64 (9, 'UTC'),
    tape String
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
