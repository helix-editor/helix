# Troubleshooting

## Healthcheck

To make sure your Helix installation is set up the way you intented it to be, make sure to run 
```
hx --health
``` 
The output is split into two sections:

### Configuration Section 

The first part of the healthcheck output consists of the locations of all the important files currently used by Helix. These include:

- the config file 
- the language file 
- the log file 
- the runtime directory

### Language Configuration Section 

The second part of the healthcheck output consists of a table which shows the current status of each language supported by Helix. There are several features which can be configured for each language. The list of the features are:

- LSP (Language Server Protocol)
- DAP (Debug Adapter Protocol)
- Highlight (???)
- TextObject (mostly comes with treesitter, [details here](./guides/textobject.md))
- Indent (mostly come with treesitter, [details here](./guides/indent.md))

The color of the entry in the table indicates the current state of the feature:

- <b style="color:red">red</b>: The feature cannot be found on the system
- <b style="color:yellow">yellow</b>: The feature isn't configured currently
- <b style="color:green">green</b>: The feature was found on the system

Additionally you can run:
```
hx --health [LANG]
```
to get a more detailed overview for the configuration status of a specific language. Additionally to the features listed above, the output contains:

- the location of the LSP binary (if available)
- the location of the DAP binary (if available)
