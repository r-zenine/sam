# ssam
ssam stands for **small scripts and aliases manager**. it is a command line tool 
that helps you manage you **scripts** and **snippets** to manage aliases and scripts.

## Features : 

* Fuzzy search command line interface based on skim to search through your aliases (think `fzf`, `peco` or `skim`). 
* Automatic generation of configuration files for **bash** and **zsh**.
* Your scripts and aliases can be in version control and still be available in your bashrc.

## Getting started :
### Initial configuration :
Fist, you want to start by creating a repository that will hold your scripts and aliases. 
Ideally, we recommend it's stucture to be as follow : 
```bash
├── aliases.yaml
└── scripts
    ├── scripts1.sh
    └── scripts2.sh
```
Once it's done, you can continue by editing a configuration file in `$HOME/.ssam_rc.toml`
that should look as follow: 

```toml
scripts_dir="/the_full_path_to_the_directory_you_created_above/scripts"
aliases_file="/the_full_path_to_the_directory_you_created_above/aliases.yaml"
```

#### Alias management:
the `aliases.yaml` file can look like this : 
```yaml
- name: my_alias
  description: a beautifull description
  alias: ls -l 
```

#### Scripts management 
`ssam` will index your scripts as follow : 

* **name**: will be the filename of you script. 
* **description**: will be the the second line of your script. 

Therefore, you scripts should look as follow : 
```sh 
#!/bin/sh (anything other interpreter can be used here.)
# some small description
...the content of the script.
```

#### Automatic aliases in bash and zsh:

if you want the alises you defined previously to be available in your shell add the following line to your `bashrc` or `zshrc`: 
```bash 
eval "$(ssam bashrc)"
```

### Using ssam:
Once the configuration is done, try launching `saam` as follow:
``` bash 
ssam run # if you did not setup your bashrc as showed previously
am # otherwise 
```
