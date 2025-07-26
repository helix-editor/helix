// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded affix "><a href="title-page.html">Helix</a></li><li class="chapter-item expanded "><a href="install.html"><strong aria-hidden="true">1.</strong> Installation</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="package-managers.html"><strong aria-hidden="true">1.1.</strong> Package Managers</a></li><li class="chapter-item expanded "><a href="building-from-source.html"><strong aria-hidden="true">1.2.</strong> Building from source</a></li></ol></li><li class="chapter-item expanded "><a href="usage.html"><strong aria-hidden="true">2.</strong> Usage</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="registers.html"><strong aria-hidden="true">2.1.</strong> Registers</a></li><li class="chapter-item expanded "><a href="surround.html"><strong aria-hidden="true">2.2.</strong> Surround</a></li><li class="chapter-item expanded "><a href="textobjects.html"><strong aria-hidden="true">2.3.</strong> Textobjects</a></li><li class="chapter-item expanded "><a href="syntax-aware-motions.html"><strong aria-hidden="true">2.4.</strong> Syntax aware motions</a></li><li class="chapter-item expanded "><a href="pickers.html"><strong aria-hidden="true">2.5.</strong> Pickers</a></li><li class="chapter-item expanded "><a href="jumplist.html"><strong aria-hidden="true">2.6.</strong> Jumplist</a></li><li class="chapter-item expanded "><a href="keymap.html"><strong aria-hidden="true">2.7.</strong> Keymap</a></li><li class="chapter-item expanded "><a href="command-line.html"><strong aria-hidden="true">2.8.</strong> Command line</a></li><li class="chapter-item expanded "><a href="commands.html"><strong aria-hidden="true">2.9.</strong> Commands</a></li><li class="chapter-item expanded "><a href="lang-support.html"><strong aria-hidden="true">2.10.</strong> Language support</a></li></ol></li><li class="chapter-item expanded "><a href="ecosystem.html"><strong aria-hidden="true">3.</strong> Ecosystem</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="from-vim.html"><strong aria-hidden="true">3.1.</strong> Migrating from Vim</a></li><li class="chapter-item expanded "><a href="other-software.html"><strong aria-hidden="true">3.2.</strong> Helix mode in other software</a></li></ol></li><li class="chapter-item expanded "><a href="configuration.html"><strong aria-hidden="true">4.</strong> Configuration</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="editor.html"><strong aria-hidden="true">4.1.</strong> Editor</a></li><li class="chapter-item expanded "><a href="themes.html"><strong aria-hidden="true">4.2.</strong> Themes</a></li><li class="chapter-item expanded "><a href="remapping.html"><strong aria-hidden="true">4.3.</strong> Key remapping</a></li><li class="chapter-item expanded "><a href="languages.html"><strong aria-hidden="true">4.4.</strong> Languages</a></li></ol></li><li class="chapter-item expanded "><a href="guides/index.html"><strong aria-hidden="true">5.</strong> Guides</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="guides/adding_languages.html"><strong aria-hidden="true">5.1.</strong> Adding languages</a></li><li class="chapter-item expanded "><a href="guides/textobject.html"><strong aria-hidden="true">5.2.</strong> Adding textobject queries</a></li><li class="chapter-item expanded "><a href="guides/indent.html"><strong aria-hidden="true">5.3.</strong> Adding indent queries</a></li><li class="chapter-item expanded "><a href="guides/injection.html"><strong aria-hidden="true">5.4.</strong> Adding injection queries</a></li><li class="chapter-item expanded "><a href="guides/tags.html"><strong aria-hidden="true">5.5.</strong> Adding tags queries</a></li><li class="chapter-item expanded "><a href="guides/rainbow_bracket_queries.html"><strong aria-hidden="true">5.6.</strong> Adding rainbow bracket queries</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString();
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
