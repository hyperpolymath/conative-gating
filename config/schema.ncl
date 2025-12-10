# Conative Gating Policy Schema
# Type definitions for policy validation

# Language configuration for tier classification
let Language = {
  name
    | doc "Language name (lowercase)"
    | String,
  extensions
    | doc "File extensions including dot (e.g., \".rs\")"
    | Array String,
  markers
    | doc "Code markers to detect language in content"
    | Array String
    | default = [],
}

# Exception rule allowing forbidden languages in specific paths
let ExceptionRule = {
  language
    | doc "Language name to allow"
    | String,
  allowed_paths
    | doc "Path prefixes where language is permitted"
    | Array String,
  reason
    | doc "Justification for exception"
    | String,
}

# Toolchain dependency rule
let ToolchainRule = {
  tool
    | doc "Tool name (e.g., npm)"
    | String,
  tool_markers
    | doc "Markers indicating tool usage"
    | Array String,
  requires
    | doc "Required companion tool"
    | String,
  requires_markers
    | doc "Markers indicating companion tool presence"
    | Array String,
}

# Forbidden pattern detection
let ForbiddenPattern = {
  name
    | doc "Pattern identifier"
    | String,
  regex
    | doc "Regular expression to match"
    | String,
  file_types
    | doc "File types to check (\"*\" for all)"
    | Array String
    | default = ["*"],
  reason
    | doc "Why this pattern is forbidden"
    | String,
}

# SLM enforcement configuration
let EnforcementConfig = {
  slm_weight
    | doc "Weight multiplier for SLM inhibition signals"
    | Number
    | default = 1.5,
  escalate_threshold
    | doc "Score threshold for SLM escalation"
    | Number
    | default = 0.4,
  block_threshold
    | doc "Score threshold for blocking"
    | Number
    | default = 0.7,
}

# Language tier classification
let LanguagePolicy = {
  tier1
    | doc "Preferred languages (always compliant)"
    | Array Language,
  tier2
    | doc "Acceptable languages (generates concern)"
    | Array Language,
  forbidden
    | doc "Blocked languages (hard violation)"
    | Array Language,
  exceptions
    | doc "Exception rules for forbidden languages"
    | Array ExceptionRule
    | default = [],
}

# Complete policy configuration
let Policy = {
  name
    | doc "Policy name"
    | String,
  languages
    | doc "Language tier configuration"
    | LanguagePolicy,
  toolchain
    | doc "Toolchain rules"
    | { rules | Array ToolchainRule | default = [] },
  patterns
    | doc "Forbidden pattern rules"
    | { forbidden_patterns | Array ForbiddenPattern | default = [] },
  enforcement
    | doc "SLM enforcement settings"
    | EnforcementConfig
    | default = {},
}

# Export types for external use
{
  Language,
  ExceptionRule,
  ToolchainRule,
  ForbiddenPattern,
  EnforcementConfig,
  LanguagePolicy,
  Policy,
}
