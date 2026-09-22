# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.ConsensusArbiterTest do
  use ExUnit.Case, async: true

  alias ConativeGating.ConsensusArbiter

  defp llm(confidence), do: %{confidence: confidence}
  defp slm(violation), do: %{violation_confidence: violation}
  defp oracle_allow, do: %{verdict: :allow}
  defp oracle_soft, do: %{verdict: {:soft_concern, :tier2_language}}
  defp oracle_hard, do: %{verdict: {:hard_violation, :forbidden_language}}

  test "hard oracle violation always blocks, regardless of other votes" do
    for confidence <- [0.0, 0.5, 0.95, 1.0], violation <- [0.0, 0.5, 1.0] do
      assert {:block, %{reason: :policy_oracle}} =
               ConsensusArbiter.decide(llm(confidence), slm(violation), oracle_hard())
    end
  end

  test "high weighted violation confidence blocks (violation 0.61 => 0.915)" do
    assert {:block, %{reason: :high_violation_confidence}} =
             ConsensusArbiter.decide(llm(0.95), slm(0.61), oracle_allow())
  end

  test "weighted score exactly at the 0.9 boundary does NOT block" do
    # 0.6 * 1.5 == 0.8999999999999999 in IEEE-754 — strictly below 0.9, so
    # the block branch must not trigger. Documented deterministic boundary.
    assert {:escalate, detail} =
             ConsensusArbiter.decide(llm(0.9), slm(0.6), oracle_allow())

    assert detail.no_go_score < 0.9
  end

  test "clear pass allows (violation 0.19 => weighted 0.285 < 0.3)" do
    assert {:allow, _} = ConsensusArbiter.decide(llm(0.95), slm(0.19), oracle_allow())
  end

  test "weighted score at/above 0.3 never allows (violation 0.21 => 0.315)" do
    assert {:escalate, _} =
             ConsensusArbiter.decide(llm(0.99), slm(0.21), oracle_allow())
  end

  test "low LLM confidence escalates even with a clean SLM vote" do
    assert {:escalate, %{go_score: 0.5}} =
             ConsensusArbiter.decide(llm(0.5), slm(0.05), oracle_allow())
  end

  test "oracle soft concern adds 0.2 to the no-go score" do
    # 0.05*1.5 + 0.2 = 0.275 < 0.3 -> allow (with high go)
    assert {:allow, _} = ConsensusArbiter.decide(llm(0.95), slm(0.05), oracle_soft())
    # 0.1*1.5 + 0.2 = 0.35 >= 0.3 -> escalate
    assert {:escalate, _} = ConsensusArbiter.decide(llm(0.95), slm(0.1), oracle_soft())
  end

  test "decisions are deterministic for identical votes" do
    first = ConsensusArbiter.decide(llm(0.87), slm(0.34), oracle_soft())
    second = ConsensusArbiter.decide(llm(0.87), slm(0.34), oracle_soft())
    assert first == second
  end
end
