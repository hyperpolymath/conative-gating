# SPDX-License-Identifier: MPL-2.0
# Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
# SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell <jonathan@hyperpolymath.org>

defmodule ConativeGating.AuditLog do
  @moduledoc """
  Durable JSONL audit sink for gating decisions.

  Guarantees:

    * **Exactly one record per decision** — one `record/2` call appends one
      line; callers must invoke it exactly once per accepted request.
    * **Flushed before acknowledged** — each record is written and
      `:file.sync`'d before the caller receives `:ok`.
    * **Rotation** — when appending would push the active file past
      `max_bytes`, the active file is renamed to `<path>.1` (single
      generation, overwriting any previous rotation) before writing.
    * **Fail-closed** — any filesystem failure returns `{:error, reason}` and
      leaves history untouched. A decision whose audit cannot persist must
      never be answered as `allow` upstream.
    * **No proposal content** — `:content`/`"content"` keys are stripped
      defensively; audit carries identifiers, votes, and metadata only.
    * **Bounded memory** — an in-memory history (default 1,000 entries) is
      kept for diagnostics, never growing beyond capacity.

  Configuration (first match wins): explicit start options, then the
  `:conative_gating` application env, then process env vars:

    * `CONATIVE_AUDIT_PATH` — JSONL file path (default
      `conative-gating-audit.jsonl`)
    * `CONATIVE_AUDIT_MAX_BYTES` — rotation threshold (default 10 MiB)
  """

  use GenServer
  require Logger

  @default_path "conative-gating-audit.jsonl"
  @default_max_bytes 10 * 1024 * 1024
  @default_history_capacity 1_000

  # -------------------------------------------------------------------------
  # Client API
  # -------------------------------------------------------------------------

  def start_link(opts \\ []) do
    {name, opts} = Keyword.pop(opts, :name, __MODULE__)
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc """
  Persist one audit entry (a JSON-encodable map). Returns `:ok` only after
  the record has been synced to disk; `{:error, reason}` otherwise.
  """
  @spec record(map(), GenServer.server()) :: :ok | {:error, term()}
  def record(entry, server \\ __MODULE__) when is_map(entry) do
    GenServer.call(server, {:record, entry})
  end

  @doc "Bounded in-memory diagnostic history (newest last)."
  @spec history(GenServer.server()) :: [map()]
  def history(server \\ __MODULE__) do
    GenServer.call(server, :history)
  end

  @doc "The effective sink configuration (diagnostics/tests)."
  @spec config(GenServer.server()) :: map()
  def config(server \\ __MODULE__) do
    GenServer.call(server, :config)
  end

  # -------------------------------------------------------------------------
  # Server
  # -------------------------------------------------------------------------

  @impl true
  def init(opts) do
    app_env = Application.get_all_env(:conative_gating)

    path =
      Keyword.get(opts, :path) ||
        Keyword.get(app_env, :audit_path) ||
        System.get_env("CONATIVE_AUDIT_PATH") ||
        @default_path

    max_bytes =
      Keyword.get(opts, :max_bytes) ||
        Keyword.get(app_env, :audit_max_bytes) ||
        case System.get_env("CONATIVE_AUDIT_MAX_BYTES") do
          nil -> @default_max_bytes
          raw -> parse_positive_integer(raw, @default_max_bytes)
        end

    history_capacity = Keyword.get(opts, :history_capacity, @default_history_capacity)

    {:ok,
     %{
       path: Path.expand(path),
       max_bytes: max_bytes,
       history_capacity: history_capacity,
       history: []
     }}
  end

  @impl true
  def handle_call({:record, entry}, _from, state) do
    sanitized = sanitize(entry)
    line = Jason.encode!(sanitized)

    case persist(state.path, state.max_bytes, line) do
      :ok ->
        # Mirror the WIRE shape (string keys) so diagnostics see exactly what
        # was persisted — no atom/string key duality for callers.
        mirrored = Jason.decode!(line)
        history = (state.history ++ [mirrored]) |> Enum.take(-state.history_capacity)
        {:reply, :ok, %{state | history: history}}

      {:error, reason} = error ->
        Logger.error("audit persistence failed (fail-closed): #{inspect(reason)}")
        {:reply, error, state}
    end
  end

  def handle_call(:history, _from, state), do: {:reply, state.history, state}

  def handle_call(:config, _from, state) do
    {:reply,
     %{
       path: state.path,
       max_bytes: state.max_bytes,
       history_capacity: state.history_capacity,
       history_size: length(state.history)
     }, state}
  end

  # -------------------------------------------------------------------------
  # Persistence
  # -------------------------------------------------------------------------

  # Write one JSONL record, rotating first when the active file would exceed
  # max_bytes. The record is synced before returning :ok.
  defp persist(path, max_bytes, line) do
    record_bytes = byte_size(line) + 1
    current_size = file_size(path)

    with :ok <- maybe_rotate(path, max_bytes, current_size + record_bytes),
         {:ok, io} <- File.open(path, [:append, :utf8, :raw]),
         :ok <- write_and_sync(io, line) do
      :ok
    else
      {:error, reason} -> {:error, reason}
    end
  end

  defp write_and_sync(io, line) do
    with :ok <- IO.binwrite(io, [line, "\n"]),
         :ok <- :file.sync(io) do
      File.close(io)
    else
      {:error, reason} ->
        File.close(io)
        {:error, reason}
    end
  end

  defp maybe_rotate(_path, max_bytes, projected_size)
       when projected_size <= max_bytes,
       do: :ok

  defp maybe_rotate(path, _max_bytes, _projected_size) do
    rotated = path <> ".1"

    if File.exists?(path) do
      File.rm(rotated)
      File.rename(path, rotated)
    else
      :ok
    end
  end

  defp file_size(path) do
    case File.stat(path) do
      {:ok, %{size: size}} -> size
      {:error, _} -> 0
    end
  end

  # Belt-and-braces removal of proposal content keys, shallow and nested one
  # level under common envelope keys, before anything touches the disk.
  defp sanitize(entry) when is_map(entry) do
    entry
    |> Map.delete(:content)
    |> Map.delete("content")
    |> Map.new(fn
      {key, value} when is_map(value) ->
        {key, value |> Map.delete(:content) |> Map.delete("content")}

      other ->
        other
    end)
  end

  defp parse_positive_integer(raw, default) do
    case Integer.parse(raw) do
      {value, ""} when value > 0 -> value
      _ -> default
    end
  end
end
