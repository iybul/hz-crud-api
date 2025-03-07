## RUST API + POSTGRES 


## To run

Make sure you have **Docker** installed 

Acquire base image

    docker pull postgres:17.4-alpine3.21
    docker pull rust:1.85.0-bookworm 

This may require

    docker login


Navigate to base directory

    docker-compose up -build

If this fails and you are running it with `sudo` don't. 
Instead:

    `sudo chown -R $(whoami) ~/.docker`

