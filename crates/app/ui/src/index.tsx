import './theme'; // sets data-theme before the first render
import './zoom'; // and the interface size
import './styles.css';

import { render } from 'solid-js/web';

import { App } from './App';

// On macOS the window's title bar is an overlay, and the traffic lights sit
// in the toolbar (see styles.css).
document.documentElement.classList.toggle('is-mac', /Mac/.test(navigator.userAgent));

render(() => <App />, document.getElementById('root')!);
