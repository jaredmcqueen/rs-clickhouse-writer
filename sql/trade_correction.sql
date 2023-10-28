CREATE TABLE
  IF NOT EXISTS trade_correction (
    symbol String,
    exchange_code String,
    original_id UInt64,
    original_price Float64,
    original_size UInt64,
    original_conditions Array (String),
    corrected_id UInt64,
    corrected_price Float64,
    corrected_size UInt64,
    corrected_conditions Array (String),
    timestamp DateTime64 (9, 'UTC'),
    tape String
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
