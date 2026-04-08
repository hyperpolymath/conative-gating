module Src.Abi.Foreign

import Src.Abi.Types

||| FFI declarations for the gating library
||| These must match the Zig implementation in ffi/zig/src/main.zig

%foreign "C:gating_init, libgating"
public export
prim_gating_init : IO AnyPtr

%foreign "C:gating_evaluate, libgating"
public export
prim_gating_evaluate : AnyPtr -> String -> IO Int

||| Safe wrapper for gating evaluation
public export
gatingEvaluate : AnyPtr -> String -> IO Verdict
gatingEvaluate handle content = do
  v <- prim_gating_evaluate handle content
  pure $ case v of
    0 => Allow
    1 => Block
    2 => Warn
    3 => Escalate
    _ => Block -- Default to block on unknown result
