# ssam
small command line tool to manage aliases and scripts.

to get started put a configuration file in you home directory at `$HOME/.ssam_rc.toml`
like this sample : 

```toml
scripts_dir="/Users/ryadzenine/workspace/oneliners/scripts"
aliases_file="/Users/ryadzenine/workspace/oneliners/aliases.yaml"
```

the aliases file can look like this : 
```yaml
- name: my_alias
  description: a beautifull description
  alias: ls -l 
```

as for the scripts. `ssam` is going to take the name of the file and try to read the second line of the file where it expects to find a comment with a small description of the script. 

if you want the alises you defined previously to be available in your shell add the following line to your bashrc : 

```bash 
eval "$(ssa bashrc)"
```

**Note**: this is only tested for bash.

now you can launch the tool using : 

``` bash 
ssa run # if you did not setup your bashrc as showed previously
am # otherwise 
```