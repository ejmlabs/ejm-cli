# Capacity source review preparation

These files are inert source review inputs for the capacity activation in
PR #30. This slice changes no executable module, command, runtime behavior or
test registration. The files deliberately live outside Cargo source roots;
they must not be described as implemented or tested CLI features in this slice.

The manifest binds each complete source file to its intended activation path
and SHA-256. PR #30 moves these exact files into Cargo's source tree and supplies
the module declarations, callers, public harness contracts and registered tests.
That activation requires its own checks and review. The staged directory is
removed by activation so the final tree has one source copy.

This separation keeps each review below the existing size limits while making
preparation and runtime activation explicit. Review these source inputs as data;
no untrusted source should execute in a privileged reviewer job.
