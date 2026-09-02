;; SPDX-License-Identifier: MPL-2.0
;; Guix development environment.
;; Usage: guix shell -D -f guix.scm

(use-modules (guix packages)
             (guix build-system gnu)
             (guix licenses)
             (gnu packages base)
             (gnu packages bash)
             (gnu packages base)
             (gnu packages java)
             (gnu packages rust)
             (gnu packages cmake)
             (gnu packages zig)
             (gnu packages golang)
             (gnu packages node)
             (gnu packages python))

(package
  (name "conative-gating")
  (version "0.1.0")
  (source #f)
  (build-system gnu-build-system)
  (inputs (list coreutils bash  make openjdk rust cmake zig go node python))
  (synopsis "conative-gating")
  (description "conative-gating — part of the hyperpolymath ecosystem.")
  (home-page "https://github.com/hyperpolymath/conative-gating")
  (license (@ (guix licenses) mpl2.0)))
