function Welcome(props) {
    if (props.show) {
        return <h1>Hello, {props.name}</h1>;
    }
    return null;
}
