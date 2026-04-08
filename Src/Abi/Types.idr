module Src.Abi.Types

import Data.List
import Data.List.Elem
import Data.String

||| Final verdict of the gating decision
public export
data Verdict = Allow | Warn | Escalate | Block

public export
Eq Verdict where
  Allow == Allow = True
  Warn == Warn = True
  Escalate == Escalate = True
  Block == Block = True
  _ == _ = False

||| Verdict ordering: Allow < Warn < Escalate < Block
public export
Ord Verdict where
  compare Allow Allow = EQ
  compare Allow _ = LT
  compare Warn Allow = GT
  compare Warn Warn = EQ
  compare Warn _ = LT
  compare Escalate Allow = GT
  compare Escalate Warn = GT
  compare Escalate Escalate = EQ
  compare Escalate Block = LT
  compare Block Block = EQ
  compare Block _ = GT

||| Language configuration rule
public export
record LanguageRule where
  constructor MkLanguageRule
  name : String
  markers : List String

||| Toolchain configuration rule
public export
record ToolchainRule where
  constructor MkToolchainRule
  tool : String
  requires : String

||| Policy configuration
public export
record Policy where
  constructor MkPolicy
  forbiddenLanguages : List LanguageRule
  toolchainRules : List ToolchainRule

||| A proposal to be evaluated
public export
record Proposal where
  constructor MkProposal
  content : String
  files : List String

||| Evidence for a policy violation
public export
data Violation : Policy -> Proposal -> Type where
  ||| Content contains markers for a forbidden language
  ForbiddenLanguage : (rule : LanguageRule) ->
                     Elem rule (forbiddenLanguages policy) ->
                     (marker : String) ->
                     Elem marker (markers rule) ->
                     (isInfixOf (toLower marker) (toLower (content proposal)) = True) ->
                     Violation policy proposal

  ||| Content contains markers for a tool without required companion
  MissingToolchain : (rule : ToolchainRule) ->
                    Elem rule (toolchainRules policy) ->
                    (isInfixOf (toLower (tool rule)) (toLower (content proposal)) = True) ->
                    (isInfixOf (toLower (requires rule)) (toLower (content proposal)) = False) ->
                    Violation policy proposal
