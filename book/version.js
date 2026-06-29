// Version banner + landing-page tile highlighting.
//
// The site hosts several channels under one domain:
//   /            latest stable release (a copy of the newest /<version>/)
//   /master/     development docs, rebuilt on every push to master
//   /<version>/  archived release snapshots
//
// CI deploys the same built HTML to each folder, so the channel can only
// be told apart at runtime from the URL path.
(function () {
  "use strict";

  var segments = window.location.pathname.split("/").filter(Boolean);
  var first = segments[0];

  var channel;
  if (first === "master") {
    channel = { kind: "nightly" };
  } else if (/^\d+\.\d+/.test(first || "")) {
    channel = { kind: "version", version: first };
  } else {
    channel = { kind: "stable" };
  }

  if (channel.kind === "nightly" || channel.kind === "version") {
    var releasePath = "/" + segments.slice(1).join("/");

    var banner = document.createElement("div");
    banner.id = "helix-version-banner";
    banner.setAttribute("role", "status");

    var link =
      '<a href="' + releasePath + '">View this page in the latest release &rarr;</a>';
    if (channel.kind === "nightly") {
      banner.className = "nightly";
      banner.innerHTML =
        "You're reading the <strong>development (master)</strong> docs &mdash; " +
        "they describe unreleased changes. " + link;
    } else {
      banner.className = "archived";
      banner.innerHTML =
        "You're reading the docs for <strong>" + channel.version +
        "</strong>, which may be outdated. " + link;
    }

    var main = document.querySelector("#mdbook-content main") || document.body;
    main.insertBefore(banner, main.firstChild);
  }

  // Landing-page chooser
  var currentHref = channel.kind === "nightly" ? "/master/" : "/";
  var tiles = document.querySelectorAll(".version-tiles .version-tile");
  for (var i = 0; i < tiles.length; i++) {
    if (tiles[i].getAttribute("href") === currentHref) {
      tiles[i].classList.add("current");
    }
  }
})();
