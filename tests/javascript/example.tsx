import type { FC } from "react";

type Props = {
    label: string;
    onClick(): void;
};

const Button: FC<Props> = ({ label, onClick }) => {
    if (onClick) {
        return <button onClick={onClick}>{label}</button>;
    }
    return null;
};

export default Button;
