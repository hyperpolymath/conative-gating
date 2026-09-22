# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.Application do
  @moduledoc """
  OTP Application for Conative Gating Consensus Arbiter.

  Starts the supervision tree for the consensus arbiter and related processes.
  """

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      # Start the Consensus Arbiter GenServer
      ConativeGating.ConsensusArbiter,
      # Durable JSONL audit sink (CONATIVE_AUDIT_PATH / CONATIVE_AUDIT_MAX_BYTES)
      {ConativeGating.AuditLog, []}
    ]

    opts = [strategy: :one_for_one, name: ConativeGating.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
