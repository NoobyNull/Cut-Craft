# Security

## Reporting a security problem

Please **do not** report security problems in a public issue, a pull request, a discussion or
the in-app **Report a Bug Here…** form. All of those are public, so anyone could read about
the problem before it is fixed.

Report it privately instead:

1. Open the repository's **Security** tab and choose **Report a vulnerability**, or go
   straight to <https://github.com/NoobyNull/Cut-Craft/security/advisories/new>.
2. Say what the problem is, which version of Cut Craft and which operating system you saw it
   on, and the steps that show it.

Only the maintainer can see the report. You will get a reply there, and the fix and the
advisory are published together once a fixed version is out.

Examples of what counts as a security problem: a way to make Cut Craft run code it should
not, an update that installs without its signature checking out, an add-on marked official
that is not, the Web API answering from the network without its token, or a file that can
read or write outside where it should.

## Supported versions

Fixes go into the newest release only. Cut Craft's built-in updater offers it, and it is
always on the repository's Releases page.
