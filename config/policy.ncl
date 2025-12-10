# Conative Gating Policy Configuration
# RSR-compliant default policy in Nickel

# Language configuration contract
let Language = {
  name | String,
  extensions | Array String,
  markers | Array String | default = [],
} in

# Exception rule contract
let ExceptionRule = {
  language | String,
  allowed_paths | Array String,
  reason | String,
} in

# Toolchain rule contract
let ToolchainRule = {
  tool | String,
  tool_markers | Array String,
  requires | String,
  requires_markers | Array String,
} in

# Forbidden pattern contract
let ForbiddenPattern = {
  name | String,
  regex | String,
  file_types | Array String | default = ["*"],
  reason | String,
} in

# Enforcement configuration contract
let EnforcementConfig = {
  slm_weight | Number | default = 1.5,
  escalate_threshold | Number | default = 0.4,
  block_threshold | Number | default = 0.7,
} in

# Complete policy contract
let Policy = {
  name | String,
  languages | {
    tier1 | Array Language,
    tier2 | Array Language,
    forbidden | Array Language,
    exceptions | Array ExceptionRule | default = [],
  },
  toolchain | {
    rules | Array ToolchainRule | default = [],
  },
  patterns | {
    forbidden_patterns | Array ForbiddenPattern | default = [],
  },
  enforcement | EnforcementConfig | default = {},
} in

# RSR Default Policy
{
  name = "RSR Default Policy",

  languages = {
    tier1 = [
      { name = "rust", extensions = [".rs"], markers = ["fn main", "impl ", "pub fn"] },
      { name = "elixir", extensions = [".ex", ".exs"], markers = ["defmodule", "def "] },
      { name = "zig", extensions = [".zig"], markers = ["const std"] },
      { name = "ada", extensions = [".adb", ".ads"], markers = ["procedure", "package"] },
      { name = "haskell", extensions = [".hs"], markers = ["module "] },
      { name = "rescript", extensions = [".res", ".resi"], markers = ["@react.component"] },
    ],

    tier2 = [
      { name = "nickel", extensions = [".ncl"], markers = [] },
      { name = "racket", extensions = [".rkt"], markers = ["#lang"] },
    ],

    forbidden = [
      { name = "typescript", extensions = [".ts", ".tsx"], markers = [": string", ": number", "interface "] },
      { name = "python", extensions = [".py"], markers = ["import ", "def "] },
      { name = "go", extensions = [".go"], markers = ["package main", "func "] },
      { name = "java", extensions = [".java"], markers = ["public class"] },
    ],

    exceptions = [
      {
        language = "python",
        allowed_paths = ["salt/", "training/"],
        reason = "Python allowed for Salt configs and ML training",
      },
    ],
  },

  toolchain = {
    rules = [
      {
        tool = "npm",
        tool_markers = ["package.json", "npm install"],
        requires = "deno",
        requires_markers = ["deno.json"],
      },
    ],
  },

  patterns = {
    forbidden_patterns = [
      {
        name = "hardcoded_secrets",
        regex = "(?i)(password|secret|api_key)\\s*=\\s*[\"'][^\"']{8,}[\"']",
        file_types = ["*"],
        reason = "Hardcoded secrets detected",
      },
    ],
  },

  enforcement = {
    slm_weight = 1.5,
    escalate_threshold = 0.4,
    block_threshold = 0.7,
  },
} | Policy
