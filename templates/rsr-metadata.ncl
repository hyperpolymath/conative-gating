# RSR Project Metadata Schema (Nickel)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Generates: AsciiDoc, Dublin Core XML, CITATION.cff, STATE.scm

{
  # Contract definitions
  Project = {
    name | String,
    description | String,
    version | String | default = "0.1.0",
    author | String | default = "Jonathan D.A. Jewell",
    email | String | default = "jonathan.jewell@gmail.com",
    license | String | default = "AGPL-3.0-or-later OR LicenseRef-Palimpsest-0.5",
    domain | String,
    tags | Array String | default = ["RSR", "Rhodium Standard"],
    created | String,
    modified | String | default = created,
    repo_url | String,

    # RSR Stack components used
    stack | {
      rescript | Bool | default = false,
      deno | Bool | default = false,
      wasm | Bool | default = false,
      rust | Bool | default = false,
      ocaml | Bool | default = false,
      haskell | Bool | default = false,
      guile | Bool | default = false,
      saltstack | Bool | default = false,
    } | default = {},

    # Framework connections
    frameworks | {
      void | Bool | default = false,  # VoID vocabulary
      dublin_core | Bool | default = true,  # Dublin Core metadata
      serum | Bool | default = false,  # Serum server integration
      robot_repo | Bool | default = true,  # robot-repo-bot managed
    } | default = {},

    # Git mirrors
    mirrors | {
      github | String | default = "https://github.com/hyperpolymath/%{name}",
      gitlab | String | default = "https://gitlab.com/hyperpolymath/%{name}",
      bitbucket | String | default = "https://bitbucket.org/hyperpolymath/%{name}",
      codeberg | String | default = "https://codeberg.org/hyperpolymath/%{name}",
    },
  },

  # AsciiDoc README generator
  gen_readme_adoc = fun project =>
    let stack_list =
      (if project.stack.rescript then ["ReScript"] else [])
      @ (if project.stack.deno then ["Deno"] else [])
      @ (if project.stack.wasm then ["WASM"] else [])
      @ (if project.stack.rust then ["Rust"] else [])
      @ (if project.stack.ocaml then ["OCaml"] else [])
      @ (if project.stack.haskell then ["Haskell"] else [])
      @ (if project.stack.guile then ["Guile"] else [])
      @ (if project.stack.saltstack then ["SaltStack"] else [])
    in
    m%"
= %{project.name}
%{project.author} <%{project.email}>
:version: %{project.version}
:toc: macro
:icons: font
:source-highlighter: rouge
:experimental:

%{project.description}

toc::[]

== Overview

%{project.name} is part of the link:https://rhodium.sh[Rhodium Standard] (RSR) ecosystem.

=== Stack

[cols="1,3"]
|===
| Component | Purpose

%{std.string.join "\n" (std.array.map (fun c => "| %{c} | ") stack_list)}
|===

== Installation

[source,bash]
----
# Clone from primary mirror
git clone %{project.mirrors.github}

# Or from alternate mirrors
git clone %{project.mirrors.gitlab}
git clone %{project.mirrors.codeberg}
----

== License

Licensed under %{project.license}.

See link:LICENSE[LICENSE] for details.

== Mirrors

* link:%{project.mirrors.github}[GitHub] (primary)
* link:%{project.mirrors.gitlab}[GitLab]
* link:%{project.mirrors.bitbucket}[Bitbucket]
* link:%{project.mirrors.codeberg}[Codeberg]

== Contributing

Contributions welcome. Please read link:CONTRIBUTING.adoc[CONTRIBUTING.adoc] first.

== Metadata

* Created: %{project.created}
* Modified: %{project.modified}
* Domain: %{project.domain}
* Tags: %{std.string.join ", " project.tags}
"%,

  # Dublin Core XML generator
  gen_dc_xml = fun project =>
    m%"<?xml version="1.0" encoding="UTF-8"?>
<metadata
  xmlns="http://purl.org/dc/elements/1.1/"
  xmlns:dcterms="http://purl.org/dc/terms/"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <title>%{project.name}</title>
  <creator>%{project.author}</creator>
  <subject>%{project.domain}</subject>
  <subject>RSR</subject>
  <description>%{project.description}</description>
  <publisher>Rhodium Standard</publisher>
  <date>%{project.modified}</date>
  <type>Software</type>
  <identifier>%{project.repo_url}</identifier>
  <language>en</language>
  <rights>%{project.license}</rights>
  <dcterms:license>https://spdx.org/licenses/%{project.license}.html</dcterms:license>
</metadata>
"%,

  # CITATION.cff generator
  gen_citation_cff = fun project =>
    m%"cff-version: 1.2.0
message: "If you use this software, please cite it as below."
title: "%{project.name}"
version: "%{project.version}"
abstract: "%{project.description}"
license: "%{project.license}"
repository-code: "%{project.repo_url}"
authors:
  - family-names: "Jewell"
    given-names: "Jonathan D.A."
    email: "%{project.email}"
keywords:
%{std.string.join "\n" (std.array.map (fun t => "  - \"%{t}\"") project.tags)}
"%,

  # STATE.scm generator (Guile)
  gen_state_scm = fun project =>
    m%";; %{project.name} STATE.scm
;; Rhodium Standard Repository State
;; SPDX-License-Identifier: %{project.license}

(define-module (rsr state %{project.name})
  #:export (project-state))

(define project-state
  '((name . "%{project.name}")
    (version . "%{project.version}")
    (description . "%{project.description}")
    (author . "%{project.author}")
    (license . "%{project.license}")
    (domain . "%{project.domain}")
    (created . "%{project.created}")
    (modified . "%{project.modified}")
    (mirrors . ((github . "%{project.mirrors.github}")
                (gitlab . "%{project.mirrors.gitlab}")
                (bitbucket . "%{project.mirrors.bitbucket}")
                (codeberg . "%{project.mirrors.codeberg}")))
    (frameworks . ((void . #%{if project.frameworks.void then "t" else "f"})
                   (dublin-core . #%{if project.frameworks.dublin_core then "t" else "f"})
                   (serum . #%{if project.frameworks.serum then "t" else "f"})
                   (robot-repo . #%{if project.frameworks.robot_repo then "t" else "f"})))))
"%,
}
