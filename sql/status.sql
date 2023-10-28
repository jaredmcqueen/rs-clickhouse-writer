CREATE TABLE
  IF NOT EXISTS status (
    symbol String,
    status_code String,
    status_message String,
    reason_code String,
    reason_message String,
    timestamp DateTime64 (9, 'UTC'),
    tape String
  ) Engine = MergeTree ()
ORDER BY
  (symbol, timestamp);
