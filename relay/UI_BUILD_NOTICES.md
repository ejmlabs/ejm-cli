# UI build notices

The proposed relay appearance consumes public packages from
[ejmlabs/ejm-ui](https://github.com/ejmlabs/ejm-ui). Their software retains its
MIT license. Supplied names, marks and brand materials retain their owners'
rights; public source access does not grant trademark rights.

Manrope and IBM Plex Mono retain the SIL Open Font License 1.1 and their
copyright notices. The static screen build copies both complete notices through public
`@ejm/ui/fonts/*` exports into its generated `notices/` directory. Preserve them
when distributing the corresponding font assets. Other build dependencies
retain their licenses and notices in the locked dependency graph.

React, ReactDOM server, Radix, Vite and Tailwind are build dependencies for this
source layer. These packages run only during the static build; no framework runtime is served by the Worker or browser.
The shared theme bootstrap is copied unchanged and reads an existing shared
theme choice or the system preference. It does not write a product preference.
The shared input is protected UI PR #11 merge
`79a1aa8b1075f834717bc0c0299dc2c41a22b966`, tree
`c18c74fd336dbad9515bff8ecf22ca980b889a51`. CLI PR #22 adopted its source pin,
readable source and install context through protected merge
`78810826cd376588d3303f13bc237309fb5170a8`. This Worker source still requires
its own normal full gate, hosted review, protected adoption and deployment.
Current local stage checks are recorded in the publication guide. Those
prerequisite merges do not establish a public application release.
