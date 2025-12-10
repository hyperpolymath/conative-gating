defmodule ConativeGating.ConsensusArbiter do
  @moduledoc """
  Consensus Arbiter for Conative Gating.

  Implements modified PBFT with asymmetric weighting that favors
  inhibition (SLM NO-GO signals) over LLM GO signals.
  """

  use GenServer

  @slm_weight 1.5  # SLM votes count 1.5x (asymmetric)

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  def init(_opts) do
    {:ok, %{decisions: [], audit_log: []}}
  end

  @doc """
  Make a consensus decision based on LLM, SLM, and Oracle evaluations.

  ## Decision Matrix

  | LLM Confidence | SLM Violation | Result   |
  |----------------|---------------|----------|
  | High (>0.8)    | Low (<0.3)    | ALLOW    |
  | High (>0.8)    | Med (0.3-0.6) | ESCALATE |
  | High (>0.8)    | High (>0.6)   | BLOCK    |
  | Med (0.5-0.8)  | Any >0.4      | ESCALATE |
  | Low (<0.5)     | Any           | ESCALATE |
  """
  def decide(llm_result, slm_result, oracle_result) do
    # Hard violations from Oracle are immediate blocks
    case oracle_result.verdict do
      {:hard_violation, type} ->
        {:block, %{reason: :policy_oracle, type: type}}

      _ ->
        weighted_decision(llm_result, slm_result, oracle_result)
    end
  end

  defp weighted_decision(llm, slm, oracle) do
    # Calculate weighted scores
    go_score = llm.confidence
    no_go_score = slm.violation_confidence * @slm_weight

    # Add oracle soft concerns to no_go
    no_go_score = case oracle.verdict do
      {:soft_concern, _} -> no_go_score + 0.2
      _ -> no_go_score
    end

    cond do
      # Clear violation
      no_go_score > 0.9 ->
        {:block, %{reason: :high_violation_confidence, slm: slm, oracle: oracle}}

      # Clear pass
      go_score > 0.8 and no_go_score < 0.3 ->
        {:allow, %{llm: llm}}

      # Uncertain - escalate to human
      true ->
        {:escalate, %{
          go_score: go_score,
          no_go_score: no_go_score,
          llm: llm,
          slm: slm,
          oracle: oracle
        }}
    end
  end
end
