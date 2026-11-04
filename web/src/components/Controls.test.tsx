import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import Controls from './Controls';

describe('Controls', () => {
  it('shows the turn and dispatches play/pause', () => {
    const dispatch = vi.fn();
    render(<Controls playback={{ turn: 0, last: 10, playing: false, speed: 30 }} dispatch={dispatch} />);
    expect(screen.getByText('Turn 0 / 10')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Play' }));
    expect(dispatch).toHaveBeenCalledWith({ type: 'toggle' });
  });

  it('disables step forward on the last turn and step back on the first', () => {
    const { unmount } = render(<Controls playback={{ turn: 10, last: 10, playing: false, speed: 30 }} dispatch={vi.fn()} />);
    expect((screen.getByRole('button', { name: 'Step forward' }) as HTMLButtonElement).disabled).toBe(true);
    unmount();
    render(<Controls playback={{ turn: 0, last: 10, playing: false, speed: 30 }} dispatch={vi.fn()} />);
    expect((screen.getByRole('button', { name: 'Step back' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('labels the play button Pause while playing', () => {
    render(<Controls playback={{ turn: 3, last: 10, playing: true, speed: 30 }} dispatch={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Pause' })).toBeTruthy();
  });
});
