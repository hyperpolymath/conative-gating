# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.ArbiterProtocol do
  @moduledoc """
  Versioned JSON-lines protocol for the Consensus Arbiter.

  Wire format (protocol version 1):

      request:
        {"protocol_version":1,"request_id":"…",
         "llm":{"confidence":0.95},
         "slm":{"violation_confidence":0.05},
         "oracle":{"verdict":"allow|soft_concern|hard_violation"}}

      response:
        {"protocol_version":1,"request_id":"…",
         "verdict":"allow|escalate|block","reason":"…","audit_recorded":true}

      service error:
        {"protocol_version":1,"request_id":"…","error":"…"}

  Decoding never raises: every malformed request yields
  `{:error, reason}` so the caller can answer with a protocol error and fail
  closed. See `docs/ARBITER_PROTOCOL.adoc`.
  """

  @protocol_version 1
  @oracle_verdicts ~w(allow soft_concern hard_violation)
  @final_verdicts ~w(allow escalate block)

  @typedoc "A validated consensus request."
  @type request :: %{
          protocol_version: 1,
          request_id: String.t(),
          llm: %{confidence: float()},
          slm: %{violation_confidence: float()},
          oracle: %{verdict: String.t()}
        }

  def protocol_version, do: @protocol_version
  def final_verdicts, do: @final_verdicts
  def oracle_verdicts, do: @oracle_verdicts

  @doc """
  Decode and validate one request line.
  """
  @spec decode_request(binary()) :: {:ok, request()} | {:error, atom() | tuple()}
  def decode_request(line) when is_binary(line) do
    with {:ok, decoded} <- Jason.decode(line),
         {:ok, request} <- validate_request(decoded) do
      {:ok, request}
    else
      {:error, %Jason.DecodeError{}} -> {:error, :malformed_json}
      {:error, reason} -> {:error, reason}
    end
  end

  defp validate_request(%{"protocol_version" => version} = request)
       when version != @protocol_version do
    _ = request
    {:error, {:unsupported_protocol_version, version}}
  end

  defp validate_request(
         %{
           "protocol_version" => @protocol_version,
           "request_id" => request_id,
           "llm" => %{"confidence" => confidence},
           "slm" => %{"violation_confidence" => violation_confidence},
           "oracle" => %{"verdict" => oracle_verdict}
         } = request
       )
       when is_binary(request_id) and is_number(confidence) and
              is_number(violation_confidence) do
    cond do
      request_id == "" ->
        {:error, :missing_request_id}

      confidence < 0 or confidence > 1 ->
        {:error, {:out_of_range, "llm.confidence"}}

      violation_confidence < 0 or violation_confidence > 1 ->
        {:error, {:out_of_range, "slm.violation_confidence"}}

      oracle_verdict not in @oracle_verdicts ->
        {:error, {:unknown_oracle_verdict, oracle_verdict}}

      true ->
        {:ok,
         %{
           protocol_version: @protocol_version,
           request_id: request_id,
           llm: %{confidence: confidence / 1},
           slm: %{violation_confidence: violation_confidence / 1},
           oracle: %{verdict: oracle_verdict}
         }}
    end
  end

  defp validate_request(_other), do: {:error, :invalid_request_shape}

  @doc "Encode a consensus response (one line, no trailing newline)."
  @spec encode_response(String.t(), String.t(), String.t() | nil, boolean()) :: binary()
  def encode_response(request_id, verdict, reason, audit_recorded)
      when verdict in @final_verdicts and is_boolean(audit_recorded) do
    Jason.encode!(%{
      protocol_version: @protocol_version,
      request_id: request_id,
      verdict: verdict,
      reason: reason,
      audit_recorded: audit_recorded
    })
  end

  @doc "Encode a service-level error response (clients must fail closed)."
  @spec encode_error(String.t() | nil, binary()) :: binary()
  def encode_error(request_id, message) do
    Jason.encode!(%{
      protocol_version: @protocol_version,
      request_id: request_id || "",
      error: message
    })
  end
end
