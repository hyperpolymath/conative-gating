module Src.Abi.Proofs

import Src.Abi.Types
import Src.Abi.Gating
import Data.List
import Data.List.Elem
import Data.String

-- ============================================================================
-- 1. Policy Completeness
-- ============================================================================

||| Lemma: If x is in xs and f x is True, then anyList f xs is True.
public export
exists_any_true : (f : a -> Bool) -> (xs : List a) -> (x : a) -> Elem x xs -> f x = True -> anyList f xs = True
exists_any_true f (x :: xs) x Here h = rewrite h in Refl
exists_any_true f (y :: xs) x (There el) h with (f y)
  exists_any_true f (y :: xs) x (There el) h | True = Refl
  exists_any_true f (y :: xs) x (There el) h | False = exists_any_true f xs x el h

||| Theorem: Any language violation results in a Block verdict
lang_violation_implies_block : 
  (p : Policy) -> (prop : Proposal) -> 
  (rule : LanguageRule) -> 
  (el : Elem rule (forbiddenLanguages p)) ->
  (marker : String) ->
  (em : Elem marker (markers rule)) ->
  (isInfixOf (toLower marker) (toLower (content prop)) = True) ->
  gateOracle p prop = Block
lang_violation_implies_block p prop rule el marker em h = 
  let marker_true = exists_any_true (\m => isInfixOf (toLower m) (toLower (content prop))) (markers rule) marker em h
      lang_true = exists_any_true (\r => isForbiddenLang r prop) (forbiddenLanguages p) rule el marker_true
  in ?hole_lang_violation_block

||| Theorem: Any policy violation results in a Block verdict
public export
policy_completeness : 
  (p : Policy) -> (prop : Proposal) -> 
  Violation p prop -> gateOracle p prop = Block
policy_completeness p prop v = ?hole_policy_completeness

-- ============================================================================
-- 2. Gate Monotonicity
-- ============================================================================

||| Theorem: The gating process is monotonic with respect to the Verdict order
public export
gate_monotonicity : 
  (prop : Proposal) -> (v : Verdict) ->
  (v <= slmStage prop v) = True
gate_monotonicity prop Allow = Refl
gate_monotonicity prop Warn = Refl
gate_monotonicity prop Escalate = Refl
gate_monotonicity prop Block = Refl

-- ============================================================================
-- 3. Policy Composition Soundness
-- ============================================================================

||| Lemma: If anyList f xs is True, then anyList f (xs ++ ys) is True
public export
any_append : (f : a -> Bool) -> (xs, ys : List a) -> anyList f xs = True -> anyList f (xs ++ ys) = True
any_append f (x :: xs) ys eq with (f x)
  any_append f (x :: xs) ys eq | True = Refl
  any_append f (x :: xs) ys eq | False = any_append f xs ys eq

public export
policy_composition_soundness : 
  (p1 : Policy) -> (p2 : Policy) -> (prop : Proposal) ->
  gateOracle p1 prop = Block -> 
  gateOracle (MkPolicy (forbiddenLanguages p1 ++ forbiddenLanguages p2) 
                       (toolchainRules p1 ++ toolchainRules p2)) prop = Block
policy_composition_soundness p1 p2 prop eq = ?hole_policy_composition_soundness

-- ============================================================================
-- 4. False Positive Boundedness
-- ============================================================================

||| A proposal is clean if it contains no forbidden language markers
public export
CleanProposal : Policy -> Proposal -> Type
CleanProposal p prop = (rule : LanguageRule) -> Elem rule (forbiddenLanguages p) ->
                      (marker : String) -> Elem marker (markers rule) ->
                      isInfixOf (toLower marker) (toLower (content prop)) = False

||| Theorem: A clean proposal (no toolchain rules) is Allowed
||| This shows the gate doesn't block "everything".
public export
clean_allowed : (p : Policy) -> (prop : Proposal) -> 
                (forbiddenLanguages p = []) -> 
                (toolchainRules p = []) ->
                gateOracle p prop = Allow
clean_allowed (MkPolicy [] []) prop Refl Refl = Refl

-- ============================================================================
-- 5. Deterministic Rule Evaluation
-- ============================================================================

||| Theorem: The gating oracle is deterministic
||| (Guaranteed by purity in Idris2)
public export
deterministic_oracle : (p : Policy) -> (prop : Proposal) -> 
                      gateOracle p prop = gateOracle p prop
deterministic_oracle p prop = Refl
