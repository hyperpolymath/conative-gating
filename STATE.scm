;;; STATE.scm --- conative-gating conversation checkpoint
;;; Format: Guile Scheme S-expressions
;;; Schema: RSR STATE v2.0
;;;
;;; SLM-as-Cerebellum for LLM Policy Enforcement
;;; with inhibitory antagonist consensus mechanism.

(define state
  `((metadata
     (format-version . "2.0")
     (schema-version . "2025-12-10")
     (project . "conative-gating")
     (created . "2025-12-09")
     (updated . "2025-12-10T21:00:00Z"))

    (position
     (summary . "SLM-as-Cerebellum for LLM Policy Enforcement")
     (phase . research-design)
     (maturity . alpha)
     (rsr-tier . 1)
     (primary-language . "rust")
     (domain . "AI Safety"))

    (context
     (last-session . "2025-12-10")
     (focus-area . "MAAF ecosystem integration")
     (blockers . ())
     (decisions-pending
      ("SLM model selection"
       "Consensus threshold tuning")))

    (architecture
     (pattern . "inhibitory-antagonist")
     (consensus . "modified-pbft")
     (asymmetry . 1.5)
     (components
      ((name . "Policy Oracle")
       (language . "rust")
       (status . planned)
       (files . ("src/oracle/")))
      ((name . "SLM Bindings")
       (language . "rust")
       (status . planned)
       (files . ("src/slm/")))
      ((name . "Arbiter")
       (language . "elixir")
       (status . planned)
       (files . ("arbiter/")))
      ((name . "Policy Config")
       (language . "nickel")
       (status . planned)
       (files . ("policy/")))))

    (predecessors
     (echomesh . archived)
     (upm . archived)
     (rhodibot . partial-archived)
     (elegant-state . simplified))

    (ecosystem
     (part-of . ("RSR Framework" "MAAF"))
     (depends-on . ("consent-aware-http" "wharf" "kith"))
     (integrates-with
      . ("indieweb2-bastion"
         "cadre-router"
         "zotero-nsai"
         "echidna"
         "fogbinder"
         "vext")))

    (issues
     (active . ("Initial implementation" "MAAF integration"))
     (resolved . ("Architecture design" "RSR standardization"))
     (known-limitations . ("SLM model not selected"))
     (technical-debt . ()))

    (roadmap
     (current-version . "0.0.1")
     (next-milestone . "Policy Oracle MVP")
     (version-plan
      ((version . "0.1.0")
       (features . ("Policy Oracle" "Nickel schema" "Basic consensus")))
      ((version . "0.2.0")
       (features . ("SLM integration" "Elixir arbiter" "AIBDP enforcement")))))

    (session-files
     ("STATE.scm"
      "docs/MAAF_INTEGRATION.adoc"
      "README.adoc"))

    (notes
     "Core policy enforcement for RSR ecosystem. Uses inhibitory antagonist
      pattern where SLM acts as cerebellum providing fast policy checks while
      larger models handle complex reasoning. Integrates with MAAF for consent
      enforcement and indieweb2-bastion for identity/provenance.

      Key MAAF integrations:
      - kith for .well-known/ management
      - consent-aware-http for HTTP 430
      - echidna for proof provenance
      - vext for IRC notifications")))

state
