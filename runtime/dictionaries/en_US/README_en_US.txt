en_US Hunspell Dictionary
Version 2026.02.25
Wed Feb 25 15:37:24 2026 -0500 [7e99eda]

https://wordlist.aspell.net

README file for English Hunspell dictionaries derived from SCOWL.

These dictionaries are created using the speller/make-hunspell-dict
script in SCOWL.

The following dictionaries are available:

  en_US (American)
  en_CA (Canadian)
  en_GB-ise (British with -ise/traditional spelling)
  en_GB-ize (British with -ize/Oxford spelling)
  en_AU (Australian)

  en_US-large
  en_CA-large
  en_GB-large (with both -ise and -ize spelling)
  en_AU-large

The default dictionaries correspond to SCOWL size 60 and, to encourage
consistent spelling, generally only include one spelling variant for a word.
The large dictionaries correspond to SCOWL size 70 and include common
spelling variants.  The larger dictionaries, however, (1) have not been as
carefully checked for errors as the normal dictionaries and thus may contain
misspelled or invalid words; and (2) contain uncommon, yet valid, words that
might cause problems as they are likely to be misspellings of more common
words (for example, "ort" and "calender").

The American, Canadian, and Australian dictionaries are considered the
official version for Hunspell.  The British ones are considered an 
alternative version.  The official ones are maintained by Marco A.G.Pinto at
https://proofingtoolgui.org.

For additional information, including information on how to contribute, see
https://wordlist.aspell.net/dicts/.

IMPORTANT CHANGES INTRODUCED ON 2026-02-22:

The Unicode "’" (U+2019) character was added to WORDCHARS so that Hunspell can
recognize words with the apostrophe.  Based on testing, this should allow
Hunspell to recognize both "can't" and "can’t".  The ASCII single quote at the
end of the word won't be considered part of the word, but the Unicode
character will.  This means "'color'" is okay, but "‘color’" will get flagged
when Hunspell does the tokenization.

IMPORTANT CHANGES INTRODUCED IN 2016.11.20:

New Australian dictionaries thanks to the work of Benjamin Titze
(btitze@protonmail.ch).

IMPORTANT CHANGES INTRODUCED IN 2016.04.24:

The dictionaries are now in UTF-8 format instead of ISO-8859-1.  This
was required to handle smart quotes correctly.

IMPORTANT CHANGES INTRODUCED IN 2016.01.19:

"SET UTF8" was changes to "SET UTF-8" in the affix file as some
versions of Hunspell do not recognize "UTF8".

ADDITIONAL NOTES:

The NOSUGGEST flag was added to certain taboo words.  While I made an
honest attempt to flag the strongest taboo words with the NOSUGGEST
flag, I MAKE NO GUARANTEE THAT I FLAGGED EVERY POSSIBLE TABOO WORD.
The list was originally derived from Németh László, however I removed
some words which, while being considered taboo by some dictionaries,
are not really considered swear words in today's society.

COPYRIGHT, SOURCES, and CREDITS:

The English dictionaries come directly from SCOWL and is thus under
the same copyright terms as SCOWL.  The affix file is a heavily
modified version of the original english.aff file, which was
released as part of Geoff Kuenning's Ispell and as such is covered by
his BSD license.

Copyright 2000-2026 by Kevin Atkinson

Permission to use, copy, modify, distribute, and sell any part of SCOWLv2, or
word lists created from it, is hereby granted without fee, provided that the
above copyright notice appears in all copies and that both the above
copyright notice and this notice appear in supporting documentation.  Kevin
Atkinson makes no representations about the suitability of this database for
any purpose.  It is provided "as is" without express or implied warranty.

SCOWL is derived from many sources, most of which are in the Public Domain.
Data from the Corpus of Contemporary American English (COCA) was also used.

All data from COCA comes from 3-gram data that is not freely available;
however, the usage is within the rights given by the NDA that was signed when
purchasing the data.  More information on COCA is available at
https://www.english-corpora.org/coca/.

The primary source of words for SCOWL comes from 12dicts and ENABLE2K.  Both
are in the Public Domain, but Alan Beale <biljir@pobox.com> deserves special
credit as he is the author of 12dicts and a major contributor to ENABLE2K.  In
addition, he gave me an incredible amount of feedback and created a number of
special lists in order to help improve the overall quality of SCOWL.

The initial information for Australian English comes from Benjamin Titze:

  Copyright 2016 by Benjamin Titze

  Permission to use, copy, modify, distribute and sell this array, the
  associated software, and its documentation for any purpose is hereby
  granted without fee, provided that the above copyright notice appears
  in all copies and that both that copyright notice and this permission
  notice appear in supporting documentation. Benjamin Titze makes no
  representations about the suitability of this array for any
  purpose. It is provided "as is" without express or implied warranty.

Affix file Copyright:

  Copyright 1993, Geoff Kuenning, Granada Hills, CA
  All rights reserved.

  Redistribution and use in source and binary forms, with or without
  modification, are permitted provided that the following conditions
  are met:

  1. Redistributions of source code must retain the above copyright
     notice, this list of conditions and the following disclaimer.
  2. Redistributions in binary form must reproduce the above copyright
     notice, this list of conditions and the following disclaimer in the
     documentation and/or other materials provided with the distribution.
  3. All modifications to the source code must be clearly marked as
     such.  Binary redistributions based on modified source code
     must be clearly marked as modified versions in the documentation
     and/or other materials provided with the distribution.
  (clause 4 removed with permission from Geoff Kuenning)
  5. The name of Geoff Kuenning may not be used to endorse or promote
     products derived from this software without specific prior
     written permission.

  THIS SOFTWARE IS PROVIDED BY GEOFF KUENNING AND CONTRIBUTORS ``AS IS'' AND
  ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
  IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
  ARE DISCLAIMED.  IN NO EVENT SHALL GEOFF KUENNING OR CONTRIBUTORS BE LIABLE
  FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
  DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
  OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
  HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
  LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
  OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
  SUCH DAMAGE.

Build Date: Wed Feb 25 15:40:53 EST 2026
Wordlist Command: mk-list --accents=strip en_US 60
