# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.ProtocolServerTest do
  use ExUnit.Case, async: false

  alias ConativeGating.{AuditLog, ProtocolServer}

  defp tmpdir!(tag) do
    dir = Path.join(System.tmp_dir!(), "conative-server-test-#{tag}-#{System.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf(dir) end)
    dir
  end

  defp start_sink!(dir) do
    {:ok, pid} = AuditLog.start_link(path: Path.join(dir, "audit.jsonl"), name: nil)

    on_exit(fn ->
      if Process.alive?(pid), do: GenServer.stop(pid)
    end)

    pid
  end

  defp request_line(request_id, votes \\ %{}) do
    Jason.encode!(%{
      protocol_version: 1,
      request_id: request_id,
      llm: %{confidence: Map.get(votes, :llm, 0.95)},
      slm: %{violation_confidence: Map.get(votes, :slm, 0.05)},
      oracle: %{verdict: Map.get(votes, :oracle, "allow")}
    })
  end

  test "allow round-trip: correlated, versioned, audited" do
    dir = tmpdir!("allow")
    sink = start_sink!(dir)

    response = ProtocolServer.process_line(request_line("req-allow-1"), sink) |> Jason.decode!()
    assert response["protocol_version"] == 1
    assert response["request_id"] == "req-allow-1"
    assert response["verdict"] == "allow"
    assert response["audit_recorded"] == true

    # Exactly one audit record for the accepted request.
    [entry] = AuditLog.history(sink)
    assert entry.request_id == "req-allow-1"
    assert entry.verdict == "allow"
  end

  test "hard oracle violation blocks" do
    dir = tmpdir!("block")
    sink = start_sink!(dir)

    response =
      ProtocolServer.process_line(request_line("req-block-1", %{oracle: "hard_violation"}), sink)
      |> Jason.decode!()

    assert response["verdict"] == "block"
    assert response["audit_recorded"] == true
  end

  test "invalid requests produce error responses, never verdicts" do
    dir = tmpdir!("invalid")
    sink = start_sink!(dir)

    bad_version =
      Jason.encode!(%{
        protocol_version: 2,
        request_id: "req-bad",
        llm: %{confidence: 0.9},
        slm: %{violation_confidence: 0.1},
        oracle: %{verdict: "allow"}
      })

    response = ProtocolServer.process_line(bad_version, sink) |> Jason.decode!()
    assert response["error"] =~ "unsupported protocol version"
    assert response["request_id"] == ""
    refute Map.has_key?(response, "verdict")

    garbage = ProtocolServer.process_line("this is not json", sink) |> Jason.decode!()
    assert garbage["error"] =~ "invalid request"

    # No audit records were created for refused requests.
    assert AuditLog.history(sink) == []
  end

  test "audit failure yields an error response, never an unaudited verdict" do
    missing_parent = Path.join(System.tmp_dir!(), "conative-missing-#{System.unique_integer([:positive])}")
    {:ok, sink} = AuditLog.start_link(path: Path.join(missing_parent, "audit.jsonl"), name: nil)

    on_exit(fn -> if Process.alive?(sink), do: GenServer.stop(sink) end)

    response = ProtocolServer.process_line(request_line("req-audit-fail"), sink) |> Jason.decode!()
    assert response["request_id"] == "req-audit-fail"
    assert response["error"] =~ "audit persistence failed"
    refute Map.has_key?(response, "verdict")
  end

  test "deterministic votes produce identical verdicts across processes" do
    dir = tmpdir!("determinism")
    sink = start_sink!(dir)

    line = request_line("req-det", %{llm: 0.87, slm: 0.34, oracle: "soft_concern"})
    first = ProtocolServer.process_line(line, sink) |> Jason.decode!()
    second = ProtocolServer.process_line(line, sink) |> Jason.decode!()
    assert first["verdict"] == second["verdict"]
  end
end
