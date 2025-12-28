# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>
# SPDX-License-Identifier: AGPL-3.0-or-later

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
      ConativeGating.ConsensusArbiter
    ]

    opts = [strategy: :one_for_one, name: ConativeGating.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
