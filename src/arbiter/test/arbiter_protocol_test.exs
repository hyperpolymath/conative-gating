# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.ArbiterProtocolTest do
  use ExUnit.Case, async: true

  alias ConativeGating.ArbiterProtocol

  defp request_json(overrides \\ %{}) do
    base = %{
      "protocol_version" => 1,
      "request_id" => "11111111-2222-3333-4444-555555555555",
      "llm" => %{"confidence" => 0.95},
      "slm" => %{"violation_confidence" => 0.05},
      "oracle" => %{"verdict" => "allow"}
    }

    Jason.encode!(deep_merge(base, overrides))
  end

  defp deep_merge(a, b) do
    Map.merge(a, b, fn _k, av, bv ->
      if is_map(av) and is_map(bv), do: Map.merge(av, bv), else: bv
    end)
  end

  test "valid request decodes with all fields" do
    assert {:ok, request} = ArbiterProtocol.decode_request(request_json())
    assert request.protocol_version == 1
    assert request.request_id == "11111111-2222-3333-4444-555555555555"
    assert request.llm.confidence == 0.95
    assert request.slm.violation_confidence == 0.05
    assert request.oracle.verdict == "allow"
  end

  test "non-JSON input is rejected" do
    assert {:error, :malformed_json} = ArbiterProtocol.decode_request("not json")
  end

  test "unsupported protocol version is rejected" do
    line = request_json(%{"protocol_version" => 2})
    assert {:error, {:unsupported_protocol_version, 2}} = ArbiterProtocol.decode_request(line)
  end

  test "out-of-range confidences are rejected" do
    line = request_json(%{"llm" => %{"confidence" => 1.5}})
    assert {:error, {:out_of_range, "llm.confidence"}} = ArbiterProtocol.decode_request(line)

    line = request_json(%{"slm" => %{"violation_confidence" => -0.1}})

    assert {:error, {:out_of_range, "slm.violation_confidence"}} =
             ArbiterProtocol.decode_request(line)
  end

  test "unknown oracle verdict is rejected" do
    line = request_json(%{"oracle" => %{"verdict" => "uncertain"}})

    assert {:error, {:unknown_oracle_verdict, "uncertain"}} =
             ArbiterProtocol.decode_request(line)
  end

  test "empty request id is rejected" do
    line = request_json(%{"request_id" => ""})
    assert {:error, :missing_request_id} = ArbiterProtocol.decode_request(line)
  end

  test "missing envelope keys are rejected" do
    line = Jason.encode!(%{"protocol_version" => 1, "request_id" => "x"})
    assert {:error, :invalid_request_shape} = ArbiterProtocol.decode_request(line)
  end

  test "integer confidences are coerced to floats" do
    line = request_json(%{"llm" => %{"confidence" => 1}})
    assert {:ok, request} = ArbiterProtocol.decode_request(line)
    assert is_float(request.llm.confidence)
  end

  test "response encoding round-trips" do
    encoded = ArbiterProtocol.encode_response("req-1", "block", "high no-go", true)
    decoded = Jason.decode!(encoded)
    assert decoded["protocol_version"] == 1
    assert decoded["request_id"] == "req-1"
    assert decoded["verdict"] == "block"
    assert decoded["reason"] == "high no-go"
    assert decoded["audit_recorded"] == true
  end

  test "error encoding carries nil request id as empty string" do
    decoded = Jason.decode!(ArbiterProtocol.encode_error(nil, "bad"))
    assert decoded["error"] == "bad"
    assert decoded["request_id"] == ""
    assert decoded["protocol_version"] == 1
  end
end
