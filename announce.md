Previously, if you had a file like this:

```html
<p>Some text 1234</p>
<script type="text/javascript">
  // More text 1234
  foo();
</script>
```

Pressing `Space + c` (toggle comment) on the JavaScript comment would've used the HTML comment token:

```html
<p>Some text 1234</p>
<script type="text/javascript">
  <!-- // More text 1234 -->
  foo();
</script>
```

This PR fixes that. Now, the comment token is properly recognized:

```hmtl
<p>
  Some text 1234
</p>
<script type="text/javascript">
  More text 1234
  foo();
</script>
```

It works in _all_ languages that properly inject comment tokens. For instance, JSX, Svelte, and others.

Closes https://github.com/helix-editor/helix/issues/7364
Closes https://github.com/helix-editor/helix/issues/11647
