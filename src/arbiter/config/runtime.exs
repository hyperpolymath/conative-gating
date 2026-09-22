# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# Runtime configuration — evaluated at boot (escript-friendly).

import Config

audit_max_bytes =
  case System.get_env("CONATIVE_AUDIT_MAX_BYTES") do
    nil ->
      nil

    raw ->
      case Integer.parse(raw) do
        {value, ""} when value > 0 -> value
        _ -> nil
      end
  end

config :conative_gating,
  audit_path: System.get_env("CONATIVE_AUDIT_PATH"),
  audit_max_bytes: audit_max_bytes

# Tests must never pollute the working directory: route the default audit
# path into the build tree where artifacts are disposable.
if config_env() == :test do
  config :conative_gating,
    audit_path:
      System.get_env("CONATIVE_AUDIT_PATH") ||
        Path.expand(Path.join([__DIR__, "..", "_build", "test", "conative-gating-audit.jsonl"]))
end
