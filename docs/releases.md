## Checklist

Helix releases are versioned in the Calendar Versioning scheme:
`YY.0M(.MICRO)`, for example `22.05` for May of 2022. In these instructions
we'll use `<tag>` as a placeholder for the tag being published.

* Merge the changelog PR
* Tag and push
    * `git tag -s -m "<tag>" -a <tag> && git push`
    * Make sure to switch to master and pull first
* Edit the `VERSION` file and change the date to the next planned release
    * Releases are planned to happen every two months, so `22.05` would change to `22.07`
* Wait for the Release CI to finish
    * It will automatically turn the git tag into a GitHub release when it uploads artifacts
* Edit the new release
    * Use `<tag>` as the title
    * Link to the changelog and release notes
* Merge the release notes PR
* Download the macos and linux binaries and update the `sha256`s in the [homebrew formula]
    * Use `sha256sum` on the downloaded `.tar.xz` files to determine the hash
* Link to the release notes in this-week-in-rust
    * [Example PR](https://github.com/rust-lang/this-week-in-rust/pull/3300)
* Post to reddit
    * [Example post](https://www.reddit.com/r/rust/comments/uzp5ze/helix_editor_2205_released/)

[homebrew formula]: https://github.com/helix-editor/homebrew-helix/blob/master/Formula/helix.rb

## Changelog Curation

The changelog is currently created manually by reading through commits in the
log since the last release. GitHub's compare view is a nice way to approach
this. For example when creating the 22.07 release notes, this compare link
may be used

```
https://github.com/helix-editor/helix/compare/22.05...master
```

Either side of the triple-dot may be replaced with an exact revision, so if
you wish to incrementally compile the changelog, you can tackle a weeks worth
or so, record the revision where you stopped, and use that as a starting point
next week:

```
https://github.com/helix-editor/helix/compare/7706a4a0d8b67b943c31d0c5f7b00d357b5d838d...master
```

A work-in-progress commit for a changelog might look like
[this example](https://github.com/helix-editor/helix/commit/831adfd4c709ca16b248799bfef19698d5175e55).

Not every PR or commit needs a blurb in the changelog. Each release section
tends to have a blurb that links to a GitHub comparison between release
versions for convenience:

> As usual, the following is a summary of each of the changes since the last
> release. For the full log, check out the git log.

Typically, small changes like dependencies or documentation updates, refactors,
or meta changes like GitHub Actions work are left out.