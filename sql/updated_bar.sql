CREATE TABLE
  IF NOT EXISTS updated_bar (
    symbol String,
    open Float64,
    high Float64,
    low Float64,
    close Float64,
    volume UInt64,
    timestamp DateTime64 (9, 'UTC'),
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
