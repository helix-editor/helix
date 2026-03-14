'use strict';

/* global default_theme, default_dark_theme, default_light_theme, hljs, ClipboardJS */

// Fix back button cache problem
window.onunload = function() { };

// Global variable, shared between modules
function playground_text(playground, hidden = true) {
    const code_block = playground.querySelector('code');

    if (window.ace && code_block.classList.contains('editable')) {
        const editor = window.ace.edit(code_block);
        return editor.getValue();
    } else if (hidden) {
        return code_block.textContent;
    } else {
        return code_block.innerText;
    }
}

(function codeSnippets() {
    function fetch_with_timeout(url, options, timeout = 6000) {
        return Promise.race([
            fetch(url, options),
            new Promise((_, reject) => setTimeout(() => reject(new Error('timeout')), timeout)),
        ]);
    }

    const playgrounds = Array.from(document.querySelectorAll('.playground'));
    if (playgrounds.length > 0) {
        fetch_with_timeout('https://play.rust-lang.org/meta/crates', {
            headers: {
                'Content-Type': 'application/json',
            },
            method: 'POST',
            mode: 'cors',
        })
            .then(response => response.json())
            .then(response => {
            // get list of crates available in the rust playground
                const playground_crates = response.crates.map(item => item['id']);
                playgrounds.forEach(block => handle_crate_list_update(block, playground_crates));
            });
    }

    function handle_crate_list_update(playground_block, playground_crates) {
        // update the play buttons after receiving the response
        update_play_button(playground_block, playground_crates);

        // and install on change listener to dynamically update ACE editors
        if (window.ace) {
            const code_block = playground_block.querySelector('code');
            if (code_block.classList.contains('editable')) {
                const editor = window.ace.edit(code_block);
                editor.addEventListener('change', () => {
                    update_play_button(playground_block, playground_crates);
                });
                // add Ctrl-Enter command to execute rust code
                editor.commands.addCommand({
                    name: 'run',
                    bindKey: {
                        win: 'Ctrl-Enter',
                        mac: 'Ctrl-Enter',
                    },
                    exec: _editor => run_rust_code(playground_block),
                });
            }
        }
    }

    // updates the visibility of play button based on `no_run` class and
    // used crates vs ones available on https://play.rust-lang.org
    function update_play_button(pre_block, playground_crates) {
        const play_button = pre_block.querySelector('.play-button');

        // skip if code is `no_run`
        if (pre_block.querySelector('code').classList.contains('no_run')) {
            play_button.classList.add('hidden');
            return;
        }

        // get list of `extern crate`'s from snippet
        const txt = playground_text(pre_block);
        const re = /extern\s+crate\s+([a-zA-Z_0-9]+)\s*;/g;
        const snippet_crates = [];
        let item;
        while (item = re.exec(txt)) {
            snippet_crates.push(item[1]);
        }

        // check if all used crates are available on play.rust-lang.org
        const all_available = snippet_crates.every(function(elem) {
            return playground_crates.indexOf(elem) > -1;
        });

        if (all_available) {
            play_button.classList.remove('hidden');
            play_button.hidden = false;
        } else {
            play_button.classList.add('hidden');
        }
    }

    function run_rust_code(code_block) {
        let result_block = code_block.querySelector('.result');
        if (!result_block) {
            result_block = document.createElement('code');
            result_block.className = 'result hljs language-bash';

            code_block.append(result_block);
        }

        const text = playground_text(code_block);
        const classes = code_block.querySelector('code').classList;
        let edition = '2015';
        classes.forEach(className => {
            if (className.startsWith('edition')) {
                edition = className.slice(7);
            }
        });
        const params = {
            version: 'stable',
            optimize: '0',
            code: text,
            edition: edition,
        };

        if (text.indexOf('#![feature') !== -1) {
            params.version = 'nightly';
        }

        result_block.innerText = 'Running...';

        fetch_with_timeout('https://play.rust-lang.org/evaluate.json', {
            headers: {
                'Content-Type': 'application/json',
            },
            method: 'POST',
            mode: 'cors',
            body: JSON.stringify(params),
        })
            .then(response => response.json())
            .then(response => {
                if (response.result.trim() === '') {
                    result_block.innerText = 'No output';
                    result_block.classList.add('result-no-output');
                } else {
                    result_block.innerText = response.result;
                    result_block.classList.remove('result-no-output');
                }
            })
            .catch(error => result_block.innerText = 'Playground Communication: ' + error.message);
    }

    // Syntax highlighting Configuration
    hljs.configure({
        tabReplace: '    ', // 4 spaces
        languages: [], // Languages used for auto-detection
    });

    const code_nodes = Array
        .from(document.querySelectorAll('code'))
        // Don't highlight `inline code` blocks in headers.
        .filter(function(node) {
            return !node.parentElement.classList.contains('header');
        });

    if (window.ace) {
        // language-rust class needs to be removed for editable
        // blocks or highlightjs will capture events
        code_nodes
            .filter(function(node) {
                return node.classList.contains('editable');
            })
            .forEach(function(block) {
                block.classList.remove('language-rust');
            });

        code_nodes
            .filter(function(node) {
                return !node.classList.contains('editable');
            })
            .forEach(function(block) {
                hljs.highlightBlock(block);
            });
    } else {
        code_nodes.forEach(function(block) {
            hljs.highlightBlock(block);
        });
    }

    // Adding the hljs class gives code blocks the color css
    // even if highlighting doesn't apply
    code_nodes.forEach(function(block) {
        block.classList.add('hljs');
    });

    Array.from(document.querySelectorAll('code.hljs')).forEach(function(block) {

        const lines = Array.from(block.querySelectorAll('.boring'));
        // If no lines were hidden, return
        if (!lines.length) {
            return;
        }
        block.classList.add('hide-boring');

        const buttons = document.createElement('div');
        buttons.className = 'buttons';
        buttons.innerHTML = '<button title="Show hidden lines" \
aria-label="Show hidden lines"></button>';
        buttons.firstChild.innerHTML = document.getElementById('fa-eye').innerHTML;

        // add expand button
        const pre_block = block.parentNode;
        pre_block.insertBefore(buttons, pre_block.firstChild);

        buttons.firstChild.addEventListener('click', function(e) {
            if (this.title === 'Show hidden lines') {
                this.innerHTML = document.getElementById('fa-eye-slash').innerHTML;
                this.title = 'Hide lines';
                this.setAttribute('aria-label', e.target.title);

                block.classList.remove('hide-boring');
            } else if (this.title === 'Hide lines') {
                this.innerHTML = document.getElementById('fa-eye').innerHTML;
                this.title = 'Show hidden lines';
                this.setAttribute('aria-label', e.target.title);

                block.classList.add('hide-boring');
            }
        });
    });

    if (window.playground_copyable) {
        Array.from(document.querySelectorAll('pre code')).forEach(function(block) {
            const pre_block = block.parentNode;
            if (!pre_block.classList.contains('playground')) {
                let buttons = pre_block.querySelector('.buttons');
                if (!buttons) {
                    buttons = document.createElement('div');
                    buttons.className = 'buttons';
                    pre_block.insertBefore(buttons, pre_block.firstChild);
                }

                const clipButton = document.createElement('button');
                clipButton.className = 'clip-button';
                clipButton.title = 'Copy to clipboard';
                clipButton.setAttribute('aria-label', clipButton.title);
                clipButton.innerHTML = '<i class="tooltiptext"></i>';

                buttons.insertBefore(clipButton, buttons.firstChild);
            }
        });
    }

    // Process playground code blocks
    Array.from(document.querySelectorAll('.playground')).forEach(function(pre_block) {
        // Add play button
        let buttons = pre_block.querySelector('.buttons');
        if (!buttons) {
            buttons = document.createElement('div');
            buttons.className = 'buttons';
            pre_block.insertBefore(buttons, pre_block.firstChild);
        }

        const runCodeButton = document.createElement('button');
        runCodeButton.className = 'play-button';
        runCodeButton.hidden = true;
        runCodeButton.title = 'Run this code';
        runCodeButton.setAttribute('aria-label', runCodeButton.title);
        runCodeButton.innerHTML = document.getElementById('fa-play').innerHTML;

        buttons.insertBefore(runCodeButton, buttons.firstChild);
        runCodeButton.addEventListener('click', () => {
            run_rust_code(pre_block);
        });

        if (window.playground_copyable) {
            const copyCodeClipboardButton = document.createElement('button');
            copyCodeClipboardButton.className = 'clip-button';
            copyCodeClipboardButton.innerHTML = '<i class="tooltiptext"></i>';
            copyCodeClipboardButton.title = 'Copy to clipboard';
            copyCodeClipboardButton.setAttribute('aria-label', copyCodeClipboardButton.title);

            buttons.insertBefore(copyCodeClipboardButton, buttons.firstChild);
        }

        const code_block = pre_block.querySelector('code');
        if (window.ace && code_block.classList.contains('editable')) {
            const undoChangesButton = document.createElement('button');
            undoChangesButton.className = 'reset-button';
            undoChangesButton.title = 'Undo changes';
            undoChangesButton.setAttribute('aria-label', undoChangesButton.title);
            undoChangesButton.innerHTML +=
                document.getElementById('fa-clock-rotate-left').innerHTML;

            buttons.insertBefore(undoChangesButton, buttons.firstChild);

            undoChangesButton.addEventListener('click', function() {
                const editor = window.ace.edit(code_block);
                editor.setValue(editor.originalCode);
                editor.clearSelection();
            });
        }
    });
})();

(function themes() {
    const html = document.querySelector('html');
    const themeToggleButton = document.getElementById('mdbook-theme-toggle');
    const themePopup = document.getElementById('mdbook-theme-list');
    const themeColorMetaTag = document.querySelector('meta[name="theme-color"]');
    const themeIds = [];
    themePopup.querySelectorAll('button.theme').forEach(function(el) {
        themeIds.push(el.id);
    });
    const stylesheets = {
        ayuHighlight: document.querySelector('#mdbook-ayu-highlight-css'),
        tomorrowNight: document.querySelector('#mdbook-tomorrow-night-css'),
        highlight: document.querySelector('#mdbook-highlight-css'),
    };

    function showThemes() {
        themePopup.style.display = 'block';
        themeToggleButton.setAttribute('aria-expanded', true);
        themePopup.querySelector('button#mdbook-theme-' + get_theme()).focus();
    }

    function updateThemeSelected() {
        themePopup.querySelectorAll('.theme-selected').forEach(function(el) {
            el.classList.remove('theme-selected');
        });
        const selected = get_saved_theme() ?? 'default_theme';
        let element = themePopup.querySelector('button#mdbook-theme-' + selected);
        if (element === null) {
            // Fall back in case there is no "Default" item.
            element = themePopup.querySelector('button#mdbook-theme-' + get_theme());
        }
        element.classList.add('theme-selected');
    }

    function hideThemes() {
        themePopup.style.display = 'none';
        themeToggleButton.setAttribute('aria-expanded', false);
        themeToggleButton.focus();
    }

    function get_saved_theme() {
        let theme = null;
        try {
            theme = localStorage.getItem('mdbook-theme');
        } catch {
            // ignore error.
        }
        return theme;
    }

    function delete_saved_theme() {
        localStorage.removeItem('mdbook-theme');
    }

    function get_theme() {
        const theme = get_saved_theme();
        if (theme === null || theme === undefined || !themeIds.includes('mdbook-theme-' + theme)) {
            if (typeof default_dark_theme === 'undefined') {
                // A customized index.hbs might not define this, so fall back to
                // old behavior of determining the default on page load.
                return default_theme;
            }
            return window.matchMedia('(prefers-color-scheme: dark)').matches
                ? default_dark_theme
                : default_light_theme;
        } else {
            return theme;
        }
    }

    let previousTheme = default_theme;
    function set_theme(theme, store = true) {
        let ace_theme;

        if (theme === 'coal' || theme === 'navy') {
            stylesheets.ayuHighlight.disabled = true;
            stylesheets.tomorrowNight.disabled = false;
            stylesheets.highlight.disabled = true;

            ace_theme = 'ace/theme/tomorrow_night';
        } else if (theme === 'ayu') {
            stylesheets.ayuHighlight.disabled = false;
            stylesheets.tomorrowNight.disabled = true;
            stylesheets.highlight.disabled = true;
            ace_theme = 'ace/theme/tomorrow_night';
        } else {
            stylesheets.ayuHighlight.disabled = true;
            stylesheets.tomorrowNight.disabled = true;
            stylesheets.highlight.disabled = false;
            ace_theme = 'ace/theme/dawn';
        }

        setTimeout(function() {
            themeColorMetaTag.content = getComputedStyle(document.documentElement).backgroundColor;
        }, 1);

        if (window.ace && window.editors) {
            window.editors.forEach(function(editor) {
                editor.setTheme(ace_theme);
            });
        }

        if (store) {
            try {
                localStorage.setItem('mdbook-theme', theme);
            } catch {
                // ignore error.
            }
        }

        html.classList.remove(previousTheme);
        html.classList.add(theme);
        previousTheme = theme;
        updateThemeSelected();
    }

    const query = window.matchMedia('(prefers-color-scheme: dark)');
    query.onchange = function() {
        set_theme(get_theme(), false);
    };

    // Set theme.
    set_theme(get_theme(), false);

    themeToggleButton.addEventListener('click', function() {
        if (themePopup.style.display === 'block') {
            hideThemes();
        } else {
            showThemes();
        }
    });

    themePopup.addEventListener('click', function(e) {
        let theme;
        if (e.target.className === 'theme') {
            theme = e.target.id;
        } else if (e.target.parentElement.className === 'theme') {
            theme = e.target.parentElement.id;
        } else {
            return;
        }
        theme = theme.replace(/^mdbook-theme-/, '');

        if (theme === 'default_theme' || theme === null) {
            delete_saved_theme();
            set_theme(get_theme(), false);
        } else {
            set_theme(theme);
        }
    });

    themePopup.addEventListener('focusout', function(e) {
        // e.relatedTarget is null in Safari and Firefox on macOS (see workaround below)
        if (!!e.relatedTarget &&
            !themeToggleButton.contains(e.relatedTarget) &&
            !themePopup.contains(e.relatedTarget)
        ) {
            hideThemes();
        }
    });

    // Should not be needed, but it works around an issue on macOS & iOS:
    // https://github.com/rust-lang/mdBook/issues/628
    document.addEventListener('click', function(e) {
        if (themePopup.style.display === 'block' &&
            !themeToggleButton.contains(e.target) &&
            !themePopup.contains(e.target)
        ) {
            hideThemes();
        }
    });

    document.addEventListener('keydown', function(e) {
        if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) {
            return;
        }
        if (!themePopup.contains(e.target)) {
            return;
        }

        let li;
        switch (e.key) {
        case 'Escape':
            e.preventDefault();
            hideThemes();
            break;
        case 'ArrowUp':
            e.preventDefault();
            li = document.activeElement.parentElement;
            if (li && li.previousElementSibling) {
                li.previousElementSibling.querySelector('button').focus();
            }
            break;
        case 'ArrowDown':
            e.preventDefault();
            li = document.activeElement.parentElement;
            if (li && li.nextElementSibling) {
                li.nextElementSibling.querySelector('button').focus();
            }
            break;
        case 'Home':
            e.preventDefault();
            themePopup.querySelector('li:first-child button').focus();
            break;
        case 'End':
            e.preventDefault();
            themePopup.querySelector('li:last-child button').focus();
            break;
        }
    });
})();

(function sidebar() {
    const sidebar = document.getElementById('mdbook-sidebar');
    const sidebarLinks = document.querySelectorAll('#mdbook-sidebar a');
    const sidebarToggleButton = document.getElementById('mdbook-sidebar-toggle');
    const sidebarResizeHandle = document.getElementById('mdbook-sidebar-resize-handle');
    const sidebarCheckbox = document.getElementById('mdbook-sidebar-toggle-anchor');
    let firstContact = null;


    /* Because we cannot change the `display` using only CSS after/before the transition, we
       need JS to do it. We change the display to prevent the browsers search to find text inside
       the collapsed sidebar. */
    if (!document.documentElement.classList.contains('sidebar-visible')) {
        sidebar.style.display = 'none';
    }
    sidebar.addEventListener('transitionend', () => {
        /* We only change the display to "none" if we're collapsing the sidebar. */
        if (!sidebarCheckbox.checked) {
            sidebar.style.display = 'none';
        }
    });
    sidebarToggleButton.addEventListener('click', () => {
        /* To allow the sidebar expansion animation, we first need to put back the display. */
        if (!sidebarCheckbox.checked) {
            sidebar.style.display = '';
            // Workaround for Safari skipping the animation when changing
            // `display` and a transform in the same event loop. This forces a
            // reflow after updating the display.
            sidebar.offsetHeight;
        }
    });

    function showSidebar() {
        document.documentElement.classList.add('sidebar-visible');
        Array.from(sidebarLinks).forEach(function(link) {
            link.setAttribute('tabIndex', 0);
        });
        sidebarToggleButton.setAttribute('aria-expanded', true);
        sidebar.setAttribute('aria-hidden', false);
        try {
            localStorage.setItem('mdbook-sidebar', 'visible');
        } catch {
            // Ignore error.
        }
    }

    function hideSidebar() {
        document.documentElement.classList.remove('sidebar-visible');
        Array.from(sidebarLinks).forEach(function(link) {
            link.setAttribute('tabIndex', -1);
        });
        sidebarToggleButton.setAttribute('aria-expanded', false);
        sidebar.setAttribute('aria-hidden', true);
        try {
            localStorage.setItem('mdbook-sidebar', 'hidden');
        } catch {
            // Ignore error.
        }
    }

    // Toggle sidebar
    sidebarCheckbox.addEventListener('change', function sidebarToggle() {
        if (sidebarCheckbox.checked) {
            const current_width = parseInt(
                document.documentElement.style.getPropertyValue('--sidebar-target-width'), 10);
            if (current_width < 150) {
                document.documentElement.style.setProperty('--sidebar-target-width', '150px');
            }
            showSidebar();
        } else {
            hideSidebar();
        }
    });

    sidebarResizeHandle.addEventListener('mousedown', initResize, false);

    function initResize() {
        window.addEventListener('mousemove', resize, false);
        window.addEventListener('mouseup', stopResize, false);
        document.documentElement.classList.add('sidebar-resizing');
    }
    function resize(e) {
        let pos = e.clientX - sidebar.offsetLeft;
        if (pos < 20) {
            hideSidebar();
        } else {
            if (!document.documentElement.classList.contains('sidebar-visible')) {
                showSidebar();
            }
            pos = Math.min(pos, window.innerWidth - 100);
            document.documentElement.style.setProperty('--sidebar-target-width', pos + 'px');
        }
    }
    //on mouseup remove windows functions mousemove & mouseup
    function stopResize() {
        document.documentElement.classList.remove('sidebar-resizing');
        window.removeEventListener('mousemove', resize, false);
        window.removeEventListener('mouseup', stopResize, false);
    }

    document.addEventListener('touchstart', function(e) {
        firstContact = {
            x: e.touches[0].clientX,
            time: Date.now(),
        };
    }, { passive: true });

    document.addEventListener('touchmove', function(e) {
        if (!firstContact) {
            return;
        }

        const curX = e.touches[0].clientX;
        const xDiff = curX - firstContact.x,
            tDiff = Date.now() - firstContact.time;

        if (tDiff < 250 && Math.abs(xDiff) >= 150) {
            if (xDiff >= 0 && firstContact.x < Math.min(document.body.clientWidth * 0.25, 300)) {
                showSidebar();
            } else if (xDiff < 0 && curX < 300) {
                hideSidebar();
            }

            firstContact = null;
        }
    }, { passive: true });
})();

(function chapterNavigation() {
    document.addEventListener('keydown', function(e) {
        if (e.altKey || e.ctrlKey || e.metaKey) {
            return;
        }
        if (window.search && window.search.hasFocus()) {
            return;
        }
        const html = document.querySelector('html');

        function next() {
            const nextButton = document.querySelector('.nav-chapters.next');
            if (nextButton) {
                window.location.href = nextButton.href;
            }
        }
        function prev() {
            const previousButton = document.querySelector('.nav-chapters.previous');
            if (previousButton) {
                window.location.href = previousButton.href;
            }
        }
        function showHelp() {
            const container = document.getElementById('mdbook-help-container');
            const overlay = document.getElementById('mdbook-help-popup');
            container.style.display = 'flex';

            // Clicking outside the popup will dismiss it.
            const mouseHandler = event => {
                if (overlay.contains(event.target)) {
                    return;
                }
                if (event.button !== 0) {
                    return;
                }
                event.preventDefault();
                event.stopPropagation();
                document.removeEventListener('mousedown', mouseHandler);
                hideHelp();
            };

            // Pressing esc will dismiss the popup.
            const escapeKeyHandler = event => {
                if (event.key === 'Escape') {
                    event.preventDefault();
                    event.stopPropagation();
                    document.removeEventListener('keydown', escapeKeyHandler, true);
                    hideHelp();
                }
            };
            document.addEventListener('keydown', escapeKeyHandler, true);
            document.getElementById('mdbook-help-container')
                .addEventListener('mousedown', mouseHandler);
        }
        function hideHelp() {
            document.getElementById('mdbook-help-container').style.display = 'none';
        }

        // Usually needs the Shift key to be pressed
        switch (e.key) {
        case '?':
            e.preventDefault();
            showHelp();
            break;
        }

        // Rest of the keys are only active when the Shift key is not pressed
        if (e.shiftKey) {
            return;
        }

        switch (e.key) {
        case 'ArrowRight':
            e.preventDefault();
            if (html.dir === 'rtl') {
                prev();
            } else {
                next();
            }
            break;
        case 'ArrowLeft':
            e.preventDefault();
            if (html.dir === 'rtl') {
                next();
            } else {
                prev();
            }
            break;
        }
    });
})();

(function clipboard() {
    const clipButtons = document.querySelectorAll('.clip-button');

    function hideTooltip(elem) {
        elem.firstChild.innerText = '';
        elem.className = 'clip-button';
    }

    function showTooltip(elem, msg) {
        elem.firstChild.innerText = msg;
        elem.className = 'clip-button tooltipped';
    }

    const clipboardSnippets = new ClipboardJS('.clip-button', {
        text: function(trigger) {
            hideTooltip(trigger);
            const playground = trigger.closest('pre');
            return playground_text(playground, false);
        },
    });

    Array.from(clipButtons).forEach(function(clipButton) {
        clipButton.addEventListener('mouseout', function(e) {
            hideTooltip(e.currentTarget);
        });
    });

    clipboardSnippets.on('success', function(e) {
        e.clearSelection();
        showTooltip(e.trigger, 'Copied!');
    });

    clipboardSnippets.on('error', function(e) {
        showTooltip(e.trigger, 'Clipboard error!');
    });
})();

(function scrollToTop() {
    const menuTitle = document.querySelector('.menu-title');

    menuTitle.addEventListener('click', function() {
        document.scrollingElement.scrollTo({ top: 0, behavior: 'smooth' });
    });
})();

(function controllMenu() {
    const menu = document.getElementById('mdbook-menu-bar');

    (function controllPosition() {
        let scrollTop = document.scrollingElement.scrollTop;
        let prevScrollTop = scrollTop;
        const minMenuY = -menu.clientHeight - 50;
        // When the script loads, the page can be at any scroll (e.g. if you refresh it).
        menu.style.top = scrollTop + 'px';
        // Same as parseInt(menu.style.top.slice(0, -2), but faster
        let topCache = menu.style.top.slice(0, -2);
        menu.classList.remove('sticky');
        let stickyCache = false; // Same as menu.classList.contains('sticky'), but faster
        document.addEventListener('scroll', function() {
            scrollTop = Math.max(document.scrollingElement.scrollTop, 0);
            // `null` means that it doesn't need to be updated
            let nextSticky = null;
            let nextTop = null;
            const scrollDown = scrollTop > prevScrollTop;
            const menuPosAbsoluteY = topCache - scrollTop;
            if (scrollDown) {
                nextSticky = false;
                if (menuPosAbsoluteY > 0) {
                    nextTop = prevScrollTop;
                }
            } else {
                if (menuPosAbsoluteY > 0) {
                    nextSticky = true;
                } else if (menuPosAbsoluteY < minMenuY) {
                    nextTop = prevScrollTop + minMenuY;
                }
            }
            if (nextSticky === true && stickyCache === false) {
                menu.classList.add('sticky');
                stickyCache = true;
            } else if (nextSticky === false && stickyCache === true) {
                menu.classList.remove('sticky');
                stickyCache = false;
            }
            if (nextTop !== null) {
                menu.style.top = nextTop + 'px';
                topCache = nextTop;
            }
            prevScrollTop = scrollTop;
        }, { passive: true });
    })();
    (function controllBorder() {
        function updateBorder() {
            if (menu.offsetTop === 0) {
                menu.classList.remove('bordered');
            } else {
                menu.classList.add('bordered');
            }
        }
        updateBorder();
        document.addEventListener('scroll', updateBorder, { passive: true });
    })();
})();
