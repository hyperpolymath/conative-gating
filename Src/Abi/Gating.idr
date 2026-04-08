module Src.Abi.Gating

import Src.Abi.Types
import Data.List
import Data.String

||| Recursive version of any for easier proofs
public export
anyList : (a -> Bool) -> List a -> Bool
anyList f [] = False
anyList f (x :: xs) = if f x then True else anyList f xs

||| Boolean check for forbidden language violations
public export
isForbiddenLang : LanguageRule -> Proposal -> Bool
isForbiddenLang rule prop = 
  anyList (\m => isInfixOf (toLower m) (toLower (content prop))) (markers rule)

||| Boolean check for toolchain violations
public export
isMissingToolchain : ToolchainRule -> Proposal -> Bool
isMissingToolchain rule prop = 
  let c = toLower (content prop)
  in (isInfixOf (toLower (tool rule)) c) && 
     not (isInfixOf (toLower (requires rule)) c)

||| The core oracle gating logic
public export
gateOracle : Policy -> Proposal -> Verdict
gateOracle policy prop = 
  if anyList (\rule => isForbiddenLang rule prop) (forbiddenLanguages policy)
  then Block
  else if anyList (\rule => isMissingToolchain rule prop) (toolchainRules policy)
  then Block
  else Allow

||| Subsequent stages (like SLM) can only maintain or increase the verdict severity.
public export
slmStage : Proposal -> Verdict -> Verdict
slmStage prop prevVerdict = 
  case prevVerdict of
    Block => Block
    Escalate => Escalate
    Warn => Warn 
    Allow => Allow
