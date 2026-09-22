# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.AuditLogTest do
  use ExUnit.Case, async: false

  alias ConativeGating.AuditLog

  defp tmpdir!(tag) do
    dir = Path.join(System.tmp_dir!(), "conative-audit-test-#{tag}-#{System.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    on_exit(fn -> File.rm_rf(dir) end)
    dir
  end

  defp start_sink(dir, opts) do
    {:ok, pid} = AuditLog.start_link(Keyword.merge([path: Path.join(dir, "audit.jsonl"), name: nil], opts))

    on_exit(fn ->
      if Process.alive?(pid), do: GenServer.stop(pid)
    end)

    pid
  end

  test "record persists one JSONL line with the entry fields" do
    dir = tmpdir!("record")
    sink = start_sink(dir, [])

    entry = %{
      audit_id: "00000000-0000-0000-0000-000000000001",
      request_id: "req-123",
      verdict: "allow",
      reason: "fixture"
    }

    assert :ok = AuditLog.record(entry, sink)
    assert :ok = AuditLog.record(%{entry | request_id: "req-124"}, sink)

    lines = dir |> Path.join("audit.jsonl") |> File.read!() |> String.split("\n", trim: true)
    assert length(lines) == 2

    first = Jason.decode!(Enum.at(lines, 0))
    assert first["request_id"] == "req-123"
    assert first["verdict"] == "allow"
  end

  test "proposal content keys are stripped before persistence" do
    dir = tmpdir!("content")
    sink = start_sink(dir, [])

    entry = %{
      request_id: "req-secret",
      verdict: "block",
      "content" => "super-secret-proposal-body-xyzzy",
      votes: %{"content" => "nested-secret-xyzzy", slm: %{violation_confidence: 0.9}}
    }

    assert :ok = AuditLog.record(entry, sink)
    raw = dir |> Path.join("audit.jsonl") |> File.read!()
    refute raw =~ "secret-proposal-body-xyzzy"
    refute raw =~ "nested-secret-xyzzy"
    assert raw =~ "req-secret"
  end

  test "rotation moves the active file to .1 once max_bytes is exceeded" do
    dir = tmpdir!("rotation")
    # Two ~120-byte records with a 200-byte budget forces one rotation.
    sink = start_sink(dir, max_bytes: 200)

    filler = String.duplicate("x", 90)
    assert :ok = AuditLog.record(%{request_id: "req-1", reason: filler}, sink)
    assert :ok = AuditLog.record(%{request_id: "req-2", reason: filler}, sink)

    path = Path.join(dir, "audit.jsonl")
    assert File.exists?(path <> ".1"), "expected rotated file audit.jsonl.1 to exist"

    rotated = File.read!(path <> ".1")
    active = File.read!(path)
    assert rotated =~ "req-1"
    refute active =~ "req-1"
    assert active =~ "req-2"

    # A third record also fits (rotation happens lazily per record).
    assert :ok = AuditLog.record(%{request_id: "req-3", reason: filler}, sink)
    lines = path |> File.read!() |> String.split("\n", trim: true)
    assert length(lines) == 2
  end

  test "persistence failure fails closed" do
    missing_parent = Path.join(System.tmp_dir!(), "conative-missing-#{System.unique_integer([:positive])}")
    bad_path = Path.join(missing_parent, "audit.jsonl")
    {:ok, sink} = AuditLog.start_link(path: bad_path, name: nil)

    on_exit(fn -> if Process.alive?(sink), do: GenServer.stop(sink) end)

    assert {:error, _reason} = AuditLog.record(%{request_id: "req-x"}, sink)
    # History must not acknowledge an unpersisted record.
    assert AuditLog.history(sink) == []
  end

  test "in-memory history is bounded by capacity" do
    dir = tmpdir!("history")
    sink = start_sink(dir, history_capacity: 5, max_bytes: 10_000_000)

    for i <- 1..7 do
      assert :ok = AuditLog.record(%{request_id: "req-#{i}"}, sink)
    end

    history = AuditLog.history(sink)
    assert length(history) == 5
    assert List.last(history)["request_id"] == "req-7"
  end
end
