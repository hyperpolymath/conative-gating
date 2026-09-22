# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.CLI do
  @moduledoc """
  Escript entry point for the Consensus Arbiter protocol server.

      conative_arbiter            # read JSONL requests on stdin, answer on stdout

  The audit sink is configured entirely through the environment
  (`CONATIVE_AUDIT_PATH`, `CONATIVE_AUDIT_MAX_BYTES` — see the AuditLog
  module). One process serves a whole stream of requests; callers that want
  process-per-request semantics (the Rust client) simply close stdin after
  one line.
  """

  alias ConativeGating.{AuditLog, ProtocolServer}

  def main(_args) do
    {:ok, _} = Application.ensure_all_started(:jason)

    case AuditLog.start_link(name: AuditLog) do
      {:ok, _pid} ->
        :ok

      {:error, {:already_started, _pid}} ->
        :ok

      {:error, reason} ->
        IO.puts(:stderr, "failed to start audit log: #{inspect(reason)}")
        exit(:audit_unavailable)
    end

    ProtocolServer.loop(AuditLog, :stdio)
  end
end
