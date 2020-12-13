# sam
![](demo.gif)

sam stands for **small aliases manager**. It is a command line tool that helps you manage your **aliases** and other common command.

Let's say you have multiple `kubernetes` clusters, runing in multiple cloud regions for different purposes, and several `namespaces`. Or, multiple kafka clusters and several `topics`. Everytime, you want to interact with one of these tools from the command line, you have to specify which region/environment/cluster/topic etc... you want your command to be apllied to. `sam` allows, you to express all your command commandes in a `templated` from and guides you to chose a value for each template variable you introduce. 

`sam` can handle dependencies between template variables ( for ex, the namespaces depende on the chosen cluster, or the kafka topics depend on the chose cluster ) and will build a dependency graph and generate a terminal user interface dynamically to prompt you to chose an appropriate value for each variable.

## Getting started :

Run `cargo run run` on the root of this repository to see a demo. 


You can also take a look at my own configuration here [r-zenine/oneliners](https://github.com/r-zenine/oneliners)

## Installing the tool 
You can download binaries for `linux` and `macos` from the release page. 
You can also use a package manager : 
### MacOS with homebrew: 
```bash
brew tap r-zenine/sam
brew install sam
```
### Ubuntu using snap
As I am still waiting for the manual validation on snap, you can only install it from the edge channel with the devmode confinment. 
```bash
snap install --edge --devmode sam
```


## How to configure the tool:
Fist, you want to start by creating a repository that will hold your scripts and aliases. 
Ideally, we recommend it's stucture to be as follow : 
```bash
your_root_directory
-------------------
        ├── aliases.yaml
        ├── vars.yaml
        ├── docker # your docker related alias for example
        │   ├── aliases.yaml
        │   └── vars.yaml
        └─── kubernetes # your kubernetes related aliases
            ├── aliases.yaml
            └── vars.yaml
```
Once it's done, you can continue by editing a configuration file in `$HOME/.sam_rc.toml`
that should look as follow: 

```toml
root_dir="./examples/oneliners/"
```

### Aliases:
The `aliases.yaml` file can look like this : 
```yaml
- name: list stuff
  desc: list current directory. 
  alias: cd {{directory}} && {{pager}} {{file}}
```
You can use the `{{ variable }}` syntax to refer to variables defined in your `vars_file`

`sam` will first prompt your for a choice for each dependant `variable`. Once this is done, it will replace each `variable` with it's corresponding choice and run the resulting command.

### Variables : 
In your `vars_file`, you can define variables. Variables can either have a static list of choices or can get their choices dynamically by running a command. The `from_command` option expects one choice per line in the output command. Each line is split by tab (\t) to extract the value and its description.

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

