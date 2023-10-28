CREATE TABLE
  IF NOT EXISTS trade (
    id UInt64,
    symbol String,
    price Float64,
    exchange_code String,
    size UInt64,
    timestamp DateTime64 (9, 'UTC'),
    condition Array (String),
    tape String
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
