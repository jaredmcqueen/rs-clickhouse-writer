CREATE TABLE
  IF NOT EXISTS luld (
    symbol String,
    limit_up Float64,
    limit_down Float64,
    indicator String,
    timestamp DateTime64 (9, 'UTC'),
    tape String
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
