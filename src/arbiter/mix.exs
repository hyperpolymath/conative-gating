# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-License-Identifier: PMPL-1.0-or-later

defmodule ConativeGating.MixProject do
  use Mix.Project

  def project do
    [
      app: :conative_gating,
      version: "0.1.0",
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      description: "Consensus Arbiter for Conative Gating",
      package: package()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {ConativeGating.Application, []}
    ]
  end

  defp deps do
    [
      {:rustler, "~> 0.30"},  # For Rust NIF integration
      {:jason, "~> 1.4"},
    ]
  end

  defp package do
    [
      licenses: ["PMPL-1.0-or-later"],
      links: %{"GitHub" => "https://github.com/hyperpolymath/conative-gating"}
    ]
  end
end
