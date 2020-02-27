
outdated:
	for p in $(dirname $(ls Cargo.toml */Cargo.toml */*/Cargo.toml)); do echo $p; (cd $p; cargo outdated -R); done


fix:
	$(for p in $(dirname $(ls Cargo.toml */Cargo.toml */*/Cargo.toml)); do echo $p; (cd $p; cargo fix --allow-dirty --broken-code --edition-idioms); done)

