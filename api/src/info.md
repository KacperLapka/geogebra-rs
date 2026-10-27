



# Important information

You need a PostgresSQL database created first
on fedora i needed to add into pg_hba.config:

```
# Type DATABASE        USER            ADDRESS                 METHOD
host    all             postgres        127.0.0.1/32            trust
host    all             postgres        ::1/128                 trust
```

first run:
> sudo -i -u postgres psql

then enter these one after another

``` 
CREATE DATABASE my_blob_store;
ALTER USER postgres WITH PASSWORD 'password';
```

everything should work after that
at least locally


for sending into the server
>  curl -X POST http://localhost:3000/api/workspaces      -H "Content-Type: application/json"      -d '{"message": "Hello from Geogebra-rs!", "coordinates": [1.5, 2.7, 3.1]}'

for requesting from the db (with an example slug i got)
> curl http://localhost:3000/api/workspaces/4WmujLes



