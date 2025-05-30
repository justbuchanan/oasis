FROM ubuntu:latest

RUN apt-get update --yes
RUN apt-get install --yes git cmake make pkg-config fontconfig pip python3-venv

COPY ./ /oasis
