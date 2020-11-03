# ssam
ssam stands for **small scripts and aliases manager**. it is a command line tool 
that helps you manage you **scripts** and **snippets** to manage aliases and scripts.

## Features : 

* Fuzzy search command line interface based on skim to search through your aliases (think `fzf`, `peco` or `skim`). 
* Your scripts and aliases can be in version control and still be available in your bashrc.
* Your aliases can be templated using variables and `ssam` will guide you and ask you to choose a value for each variable before runing your alias. 


## Getting started :

Run `cargo run run` on the root of this repository to see a demo. 

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
scripts_dir="./examples/oneliners/scripts"
aliases_file="./examples/oneliners/aliases.yaml"
vars_file="./examples/oneliners/vars.yaml"
```

#### Alias management:
the `aliases.yaml` file can look like this : 
```yaml
- name: list stuff
  desc: list current directory. 
  alias: cd {{directory}} && {{pager}} {{file}}
```
you can use the `{{ variable }}` syntax to refer to variables defined in your `vars_file`

`ssam` will first prompt your for a choice for each dependant `variable`. Once this is done, it will replace each `variable` with it's corresponding choice and run the resulting command.

#### Variables : 
in your `vars_file`, you can define variables. variables can either have a static list of choices or can get their choices dynamically by running a command. the `from_command` option expects one choice per line in the output command.

```yaml
- name: directory
  desc: an example variable
  choices:
    - value: /etc/default
      desc: etc default directory
    - value: /etc
      desc: etc directory

- name: pager
  desc: the pager tool to use
  choices: 
    - value: less
      desc: use less
    - value: cat
      desc: use cat


- name: file
  desc: file selection
  from_command: ls -1 {{ directory }}
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

