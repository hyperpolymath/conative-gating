# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.ProtocolServer do
  @moduledoc """
  Line-oriented stdio driver for the Consensus Arbiter (protocol v1).

  Reads one JSON request per line from stdin, reaches a consensus decision,
  persists the audit record, and writes exactly one response line per request
  before the requester considers the decision acknowledged:

    * malformed requests → service error response (never a verdict)
    * consensus → audit record first; an audit failure yields an error
      response, so an unaudited decision can never be acknowledged
    * EOF closes the loop

  The Rust client (`gating_contract::ArbiterClient`) spawns one short-lived
  arbiter per decision; this loop additionally tolerates multi-request
  streams for supervisor/library use.
  """

  alias ConativeGating.{ArbiterProtocol, AuditLog, ConsensusArbiter}

  @doc "Read requests until EOF, answering one line per request."
  def loop(audit_server \\ AuditLog, io \\ :stdio) do
    case IO.gets(io, "") do
      :eof ->
        :ok

      {:error, _reason} ->
        :ok

      line ->
        IO.puts(io, process_line(line, audit_server))
        loop(audit_server, io)
    end
  end

  @doc """
  Process exactly one request line and return the response line.

  Pure with respect to decision making: identical votes always produce the
  same verdict. The only side effect is the audit write (which must succeed
  for a verdict response to be produced).
  """
  def process_line(line, audit_server \\ AuditLog) do
    case ArbiterProtocol.decode_request(line) do
      {:ok, request} ->
        answer(request, audit_server)

      {:error, reason} ->
        ArbiterProtocol.encode_error(nil, "invalid request: #{format_reason(reason)}")
    end
  end

  defp answer(request, audit_server) do
    llm = %{confidence: request.llm.confidence}
    slm = %{violation_confidence: request.slm.violation_confidence}
    oracle = %{verdict: oracle_verdict(request.oracle.verdict)}

    {verdict, detail} = ConsensusArbiter.decide(llm, slm, oracle)
    verdict_text = verdict_to_text(verdict)
    reason_text = reason_text(detail)

    entry = %{
      schema: "conative-gating-audit-v1",
      audit_id: uuid4(),
      request_id: request.request_id,
      timestamp: DateTime.utc_now() |> DateTime.truncate(:second) |> DateTime.to_iso8601(),
      votes: %{
        llm: %{confidence: request.llm.confidence},
        slm: %{violation_confidence: request.slm.violation_confidence},
        oracle: %{verdict: request.oracle.verdict}
      },
      verdict: verdict_text,
      reason: reason_text,
      protocol_version: ArbiterProtocol.protocol_version()
    }

    case AuditLog.record(entry, audit_server) do
      :ok ->
        ArbiterProtocol.encode_response(request.request_id, verdict_text, reason_text, true)

      {:error, reason} ->
        ArbiterProtocol.encode_error(
          request.request_id,
          "audit persistence failed: #{format_reason(reason)}"
        )
    end
  end

  defp oracle_verdict("allow"), do: :allow
  defp oracle_verdict("soft_concern"), do: {:soft_concern, :policy_oracle}
  defp oracle_verdict("hard_violation"), do: {:hard_violation, :policy_oracle}

  defp verdict_to_text(:block), do: "block"
  defp verdict_to_text(:escalate), do: "escalate"
  defp verdict_to_text(:allow), do: "allow"

  defp reason_text(%{reason: reason}) when is_atom(reason), do: Atom.to_string(reason)
  defp reason_text(%{reason: reason}) when is_binary(reason), do: reason

  defp reason_text(detail) when is_map(detail) do
    "go=#{format_number(Map.get(detail, :go_score))} " <>
      "no_go=#{format_number(Map.get(detail, :no_go_score))}"
  end

  defp format_number(nil), do: "n/a"
  defp format_number(v) when is_float(v), do: :erlang.float_to_binary(v, decimals: 3)
  defp format_number(v), do: to_string(v)

  # Local RFC 4122 UUIDv4 (random) — the project deliberately carries no UUID
  # dependency, and Erlang/OTP has none built in.
  defp uuid4() do
    <<a::32, b::16, c0::16, d0::16, e::48>> = :crypto.strong_rand_bytes(16)
    c = Bitwise.bor(Bitwise.band(c0, 0x0FFF), 0x4000)
    d = Bitwise.bor(Bitwise.band(d0, 0x3FFF), 0x8000)
    Enum.join([hex(a, 8), hex(b, 4), hex(c, 4), hex(d, 4), hex(e, 12)], "-")
  end

  defp hex(value, width) do
    :io_lib.format("~*.16.0b", [width, value])
    |> IO.iodata_to_binary()
    |> String.downcase()
  end

  defp format_reason({:unsupported_protocol_version, v}), do: "unsupported protocol version #{v}"
  defp format_reason({:out_of_range, field}), do: "#{field} out of range 0..1"
  defp format_reason({:unknown_oracle_verdict, v}), do: "unknown oracle verdict #{v}"
  defp format_reason(reason) when is_atom(reason), do: Atom.to_string(reason)
  defp format_reason(reason), do: inspect(reason)
end
